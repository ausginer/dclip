//! The socket server: the lock, the socket path, the accept loop, and one
//! worker per connection.

use crate::{
    Result,
    cli::ServeOptions,
    clipboard,
    protocol::{process_request, write_response},
    sys,
};
use std::{
    collections::HashSet,
    env, fs, io,
    os::{
        fd::AsFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::UnixListener,
        },
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

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
        stream.set_read_timeout(Some(Duration::from_secs(12)))?;
        stream.set_write_timeout(Some(Duration::from_secs(12)))?;
        let uid = match sys::peer_uid(&stream) {
            Ok(uid) => uid,
            Err(error) => {
                let _ = write_response(&stream, Err(error.into()));
                continue;
            }
        };
        if !allowed.contains(&uid) {
            let _ = write_response(
                &stream,
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
                &stream,
                Err("too many concurrent clipboard requests".into()),
            );
            continue;
        }
        let active = active.clone();
        thread::spawn(move || {
            let _guard = ActiveGuard(active);
            let result = process_request(&stream);
            let _ = write_response(&stream, result);
        });
    }
    // lock, socket path and watcher are cleaned up by their guards.
    Ok(())
}
