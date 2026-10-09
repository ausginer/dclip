//! The socket server: the lock, the socket path, the accept loop, and one
//! worker per connection.

use crate::{
    Result,
    cli::ServeOptions,
    clipboard,
    process::Tool,
    protocol::{read_request, respond_with, write_response},
    sys::{self, Interest, Poll},
};
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{self, IoSlice, Read, Write},
    os::{
        fd::AsFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// How long a connection may take to send its request, and separately to take
/// its response.
const PHASE: Duration = Duration::from_secs(12);

/// A connection whose reads and writes share one deadline. A socket timeout
/// bounds a single call, and the kernel re-arms it within one large write, so
/// the stream is non-blocking and every wait is a `poll` for the time that
/// remains: a peer that trickles cannot stretch a phase past its budget.
pub(crate) struct Deadline<'a> {
    stream: &'a UnixStream,
    end: Instant,
}

impl<'a> Deadline<'a> {
    /// Starts the clock: everything done through this value ends within
    /// `budget` of now. `stream` must be non-blocking.
    pub(crate) fn new(stream: &'a UnixStream, budget: Duration) -> Self {
        Self {
            stream,
            end: Instant::now() + budget,
        }
    }

    /// Retries `call` until it does not block, waiting for `interest` in between.
    fn run<T>(&self, interest: Interest, mut call: impl FnMut() -> io::Result<T>) -> io::Result<T> {
        loop {
            match call() {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                outcome => return outcome,
            }
            let remaining = self.end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match sys::poll(
                &mut [Poll::new(self.stream.as_fd(), interest)],
                Some(remaining),
            ) {
                Err(error) if error.kind() != io::ErrorKind::Interrupted => return Err(error),
                _ => {}
            }
        }
    }
}

impl Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.run(Interest::Read, || (&*self.stream).read(buf))
    }
}

impl Write for Deadline<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.run(Interest::Write, || (&*self.stream).write(buf))
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.run(Interest::Write, || (&*self.stream).write_vectored(bufs))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The most connections served at once.
const WORKERS: usize = 16;

/// The bound socket. Dropping it removes the path, and only then closes the
/// listener.
struct Socket {
    path: PathBuf,
    listener: UnixListener,
}

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// What `serve` holds for as long as it runs. Fields drop in declaration
/// order, and that order is the cleanup order: the watcher stops first, then
/// the socket path is removed and the listener closes, and the lock is
/// released last. An instance that unlocked before removing its path could
/// delete the socket a successor had just bound there.
struct Bridge {
    _watcher: Option<Tool>,
    socket: Socket,
    _lock: File,
}

/// One of the [`WORKERS`] places, released when the connection holding it is
/// done.
struct Slot<'a>(&'a AtomicUsize);

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Admits a connection whose peer is allowed, while a place is free. A refusal
/// is the message the peer gets.
fn admit<'a>(
    stream: &UnixStream,
    allowed: &HashSet<u32>,
    active: &'a AtomicUsize,
) -> Result<Slot<'a>> {
    let uid = sys::peer_uid(stream)?;
    if !allowed.contains(&uid) {
        return Err(
            format!("host UID {uid} is not allowed; add --allow-uid {uid} on Fedora").into(),
        );
    }
    let slot = Slot(active);
    if active.fetch_add(1, Ordering::Relaxed) >= WORKERS {
        return Err("too many concurrent clipboard requests".into());
    }
    Ok(slot)
}

/// Serves one admitted connection: its request, then its response, each within
/// its own [`PHASE`].
fn work(stream: &UnixStream) {
    let result = read_request(Deadline::new(stream, PHASE))
        .and_then(|request| respond_with(&request, clipboard::wayland));
    let _ = write_response(Deadline::new(stream, PHASE), result);
}

pub(crate) fn serve(options: &ServeOptions) -> Result<()> {
    let uid = sys::uid();
    let mut allowed = HashSet::from([uid, 0]);
    allowed.extend(&options.allow_uids);
    let executable = env::current_exe()?;
    let path = env::var_os("CLAUDE_CLIPBOARD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            executable
                .parent()
                .unwrap_or(Path::new("."))
                .join("clipboard.sock")
        });
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path.with_extension("lock"))?;
    if lock.try_lock().is_err() {
        return Err("bridge is already running or cannot acquire its lock".into());
    }
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.file_type().is_socket() || metadata.uid() != uid {
                return Err("refusing to replace a non-owned socket path".into());
            }
            fs::remove_file(&path)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let listener = UnixListener::bind(&path)?;
    let socket = Socket { path, listener };
    // Peer UID is checked on every connection; no arbitrary clipboard writes.
    fs::set_permissions(&socket.path, fs::Permissions::from_mode(0o666))?;
    socket.listener.set_nonblocking(true)?;
    let wake = sys::wake_on_termination()?;
    let watcher = if options.sync_text {
        Some(clipboard::watch(&executable)?)
    } else {
        None
    };
    let bridge = Bridge {
        _watcher: watcher,
        socket,
        _lock: lock,
    };
    println!(
        "Clipboard bridge: {}; allowed host UIDs: {:?}",
        bridge.socket.path.display(),
        allowed
    );
    let listener = &bridge.socket.listener;
    let active = AtomicUsize::new(0);
    // The scope joins every worker before it returns, so no tool started for a
    // connection outlives `serve`. Each worker is bounded by its deadlines, so
    // the join is too.
    thread::scope(|scope| {
        loop {
            let mut ready = [
                Poll::new(listener.as_fd(), Interest::Read),
                Poll::new(wake.as_fd(), Interest::Read),
            ];
            match sys::poll(&mut ready, None) {
                Err(error) if error.kind() != io::ErrorKind::Interrupted => {
                    return Err(error.into());
                }
                _ => {}
            }
            if ready[1].ready() {
                return Ok(());
            }
            // Nothing that concerns one connection ends the loop: only an
            // `accept` failure that is not about the connection does.
            let stream = match listener.accept() {
                Ok((stream, _)) => stream,
                // The readiness was taken by a connection that vanished. The
                // listener is non-blocking, so `accept` never sleeps and a
                // signal cannot interrupt it.
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::ConnectionAborted
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let admitted = stream
                .set_nonblocking(true)
                .map_err(Into::into)
                .and_then(|()| admit(&stream, &allowed, &active));
            let slot = match admitted {
                Ok(slot) => slot,
                Err(error) => {
                    let _ = write_response(Deadline::new(&stream, PHASE), Err(error));
                    continue;
                }
            };
            // Shared so that a worker that cannot start leaves the stream here
            // to carry its refusal.
            let stream = Arc::new(stream);
            let worker = Arc::clone(&stream);
            let started = thread::Builder::new().spawn_scoped(scope, move || {
                let _slot = slot;
                work(&worker);
            });
            if let Err(error) = started {
                let _ = write_response(Deadline::new(&stream, PHASE), Err(error.into()));
            }
        }
    })
}

#[cfg(test)]
mod tests;
