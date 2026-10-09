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
    env, fs,
    io::{self, IoSlice, Read, Write},
    os::{
        fd::{AsFd, BorrowedFd},
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
/// cannot give one, not even for a single call: the kernel takes it afresh for
/// each buffer it allocates within a call, so a peer that keeps draining
/// slowly never lets one large write time out. The stream is non-blocking
/// instead, and every wait is a `poll` for the time that remains, so a peer
/// that trickles cannot stretch a phase past its budget.
pub(crate) struct Deadline<'a> {
    stream: &'a UnixStream,
    end: Instant,
    /// Ends the phase early once readable.
    stop: Option<BorrowedFd<'a>>,
}

impl<'a> Deadline<'a> {
    /// Starts the clock: everything done through this value ends within
    /// `budget` of now. `stream` must be non-blocking.
    pub(crate) fn new(stream: &'a UnixStream, budget: Duration) -> Self {
        Self {
            stream,
            end: Instant::now() + budget,
            stop: None,
        }
    }

    /// Also ends the phase, with `bridge is shutting down`, once `stop` is
    /// readable, as the wake pipe is from shutdown on. Only a wait ends early:
    /// what the peer has already sent is still read.
    pub(crate) fn or_until_shutdown(self, stop: BorrowedFd<'a>) -> Self {
        Self {
            stop: Some(stop),
            ..self
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
            let mut fds = [
                Poll::new(self.stream.as_fd(), interest),
                Poll::optional(self.stop, Interest::Read),
            ];
            match sys::poll(&mut fds, Some(remaining)) {
                Err(error) if error.kind() != io::ErrorKind::Interrupted => return Err(error),
                _ => {}
            }
            if fds[1].ready() {
                return Err(io::Error::other("bridge is shutting down"));
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

/// What advertises the bridge, held until shutdown is requested and no
/// longer. Fields drop in declaration order, and that order is the cleanup
/// order: the watcher stops first, then the socket path is removed, then the
/// listener closes, which resets any connection still queued on it. The lock
/// is not here: it is released after every worker has finished.
struct Bridge {
    _watcher: Option<Tool>,
    socket: Socket,
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
/// its own [`PHASE`]. A request still arriving when `shutdown` becomes
/// readable is turned away; one that has arrived is answered.
fn work(stream: &UnixStream, shutdown: BorrowedFd<'_>) {
    let result = read_request(Deadline::new(stream, PHASE).or_until_shutdown(shutdown))
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
    };
    println!(
        "Clipboard bridge: {}; allowed host UIDs: {:?}",
        bridge.socket.path.display(),
        allowed
    );
    let active = AtomicUsize::new(0);
    // The scope joins every worker before it returns, so no tool started for a
    // connection outlives `serve`. Each worker is bounded by its deadlines, and
    // one still waiting for its request stops waiting at shutdown, so the join
    // is bounded too.
    let served = thread::scope(|scope| {
        // Moved in, so it is dropped as the loop ends, before the scope joins
        // a single worker: the bridge stops advertising itself first.
        let bridge = bridge;
        let listener = &bridge.socket.listener;
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
                    // If the stream could not be made non-blocking, this
                    // breaks `Deadline`'s precondition and the write may
                    // block. It never does: a refusal is one short line, and
                    // nothing has been written to this stream before it.
                    let _ = write_response(Deadline::new(&stream, PHASE), Err(error));
                    continue;
                }
            };
            // Shared so that a worker that cannot start leaves the stream here
            // to carry its refusal.
            let stream = Arc::new(stream);
            let worker = Arc::clone(&stream);
            let shutdown = wake.as_fd();
            let started = thread::Builder::new().spawn_scoped(scope, move || {
                let _slot = slot;
                work(&worker, shutdown);
            });
            if let Err(error) = started {
                let _ = write_response(Deadline::new(&stream, PHASE), Err(error.into()));
            }
        }
    });
    // Released only now that every worker has finished, so a successor started
    // during the drain fails on the lock as it would against a running bridge,
    // and an instance never unlocks before removing its path, which could
    // delete the socket a successor had just bound there.
    drop(lock);
    served
}

#[cfg(test)]
mod tests;
