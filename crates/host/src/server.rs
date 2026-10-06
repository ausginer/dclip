//! The socket server: the lock, the socket path, the accept loop, and one
//! worker per connection.

use crate::{
    Result,
    cli::ServeOptions,
    clipboard,
    protocol::{read_request, respond_with, write_response},
    sys,
};
use std::{
    collections::HashSet,
    env, fs,
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
    fn run<T>(
        &self,
        interest: sys::Interest,
        mut call: impl FnMut() -> io::Result<T>,
    ) -> io::Result<T> {
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
                &mut [sys::Poll::new(self.stream.as_fd(), interest)],
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
        self.run(sys::Interest::Read, || (&*self.stream).read(buf))
    }
}

impl Write for Deadline<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.run(sys::Interest::Write, || (&*self.stream).write(buf))
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.run(sys::Interest::Write, || {
            (&*self.stream).write_vectored(bufs)
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Removes the socket path when dropped.
struct SocketGuard(PathBuf);
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

struct ActiveGuard(Arc<AtomicUsize>);
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
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
    let _socket_guard = SocketGuard(path.clone());
    // Peer UID is checked on every connection; no arbitrary clipboard writes.
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666))?;
    listener.set_nonblocking(true)?;
    sys::stop_on_termination()?;
    let _watcher = if options.sync_text {
        Some(clipboard::watch(&executable)?)
    } else {
        None
    };
    println!(
        "Clipboard bridge: {}; allowed host UIDs: {:?}",
        path.display(),
        allowed
    );
    let allowed = Arc::new(allowed);
    let active = Arc::new(AtomicUsize::new(0));
    while sys::running() {
        // A signal can reach a worker thread instead of the listener thread.
        match sys::poll_readable(listener.as_fd(), Duration::from_millis(1000)) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
        if !sys::running() {
            break;
        }
        let (stream, _) = match listener.accept() {
            Ok(client) => client,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
            Err(error) => return Err(error.into()),
        };
        if let Err(error) = stream.set_nonblocking(true) {
            let _ = write_response(&stream, Err(error.into()));
            continue;
        }
        let uid = match sys::peer_uid(&stream) {
            Ok(uid) => uid,
            Err(error) => {
                let _ = write_response(Deadline::new(&stream, PHASE), Err(error.into()));
                continue;
            }
        };
        if !allowed.contains(&uid) {
            let _ = write_response(
                Deadline::new(&stream, PHASE),
                Err(
                    format!("host UID {uid} is not allowed; add --allow-uid {uid} on Fedora")
                        .into(),
                ),
            );
            continue;
        }
        if active.fetch_add(1, Ordering::Relaxed) >= 16 {
            active.fetch_sub(1, Ordering::Relaxed);
            let _ = write_response(
                Deadline::new(&stream, PHASE),
                Err("too many concurrent clipboard requests".into()),
            );
            continue;
        }
        let active = active.clone();
        thread::spawn(move || {
            let _guard = ActiveGuard(active);
            let result = read_request(Deadline::new(&stream, PHASE))
                .and_then(|request| respond_with(&request, clipboard::wayland));
            let _ = write_response(Deadline::new(&stream, PHASE), result);
        });
    }
    // lock, socket path and watcher are cleaned up by their guards.
    Ok(())
}

#[cfg(test)]
mod tests;
