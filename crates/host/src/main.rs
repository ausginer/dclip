use serde_json::{Value, json};
use std::{
    collections::HashSet,
    env, fs,
    io::{self, BufRead, BufReader, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const LIMIT: usize = 64 * 1024 * 1024;
const FORMATS: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/jpg",
    "image/gif",
    "image/webp",
];
static RUNNING: AtomicBool = AtomicBool::new(true);
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

extern "C" fn stop(_: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

struct ChildGuard(Child, bool);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.1 {
            return;
        }
        // Each tool is started in its own process group. Reap on every path.
        unsafe {
            libc::kill(-(self.0.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn capture(
    program: &str,
    args: &[&str],
    input: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut child = ChildGuard(
        Command::new(program)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()?,
        true,
    );
    let mut stdout = child.0.stdout.take().ok_or("missing tool stdout")?;
    let fd = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error().into());
    }
    let writer = match input {
        Some(bytes) => {
            let mut stdin = child.0.stdin.take().ok_or("missing tool stdin")?;
            Some(thread::spawn(move || stdin.write_all(&bytes)))
        }
        None => None,
    };
    let started = Instant::now();
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 16 * 1024];
    let mut eof = false;
    let outcome: Result<()> = (|| {
        loop {
            if started.elapsed() >= timeout {
                return Err(format!("{program} timed out").into());
            }
            if !eof {
                match stdout.read(&mut chunk) {
                    Ok(0) => eof = true,
                    Ok(n) => {
                        if bytes.len() + n > LIMIT {
                            return Err("clipboard exceeds 64 MiB".into());
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            if let Some(status) = child.0.try_wait()? {
                if !status.success() {
                    return Err(format!("{program} failed; check it on Fedora").into());
                }
                if eof {
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
    })();
    // Kill before joining a possibly blocked stdin writer on failure.
    if outcome.is_ok() {
        child.1 = false;
    }
    drop(child);
    if let Some(writer) = writer {
        let written = writer.join().map_err(|_| "clipboard writer panicked")?;
        if outcome.is_ok() {
            written?;
        }
    }
    outcome?;
    Ok(bytes)
}

fn clipboard(args: &[&str]) -> Result<Vec<u8>> {
    capture("wl-paste", args, None, Duration::from_secs(4))
}

fn image_valid(mime: &str, bytes: &[u8]) -> bool {
    match mime {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" | "image/jpg" => bytes.starts_with(b"\xff\xd8\xff"),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()),
        _ => false,
    }
}

fn respond_with<F>(request: &[u8], mut clipboard: F) -> Result<Vec<u8>>
where
    F: FnMut(&[&str]) -> Result<Vec<u8>>,
{
    let request: Value = serde_json::from_slice(request)?;
    let object = request.as_object().ok_or("request must be an object")?;
    if object.keys().any(|key| key != "op" && key != "type") {
        return Err("unsupported request field".into());
    }
    let op = object
        .get("op")
        .and_then(Value::as_str)
        .ok_or("missing operation")?;
    if !matches!(op, "types" | "read") {
        return Err("unsupported operation".into());
    }
    let raw_types = clipboard(&["--list-types"])?;
    let offered = std::str::from_utf8(&raw_types)?
        .lines()
        .collect::<HashSet<_>>();
    let images = FORMATS
        .iter()
        .copied()
        .filter(|mime| offered.contains(mime))
        .collect::<Vec<_>>();
    if op == "types" {
        return Ok(if images.is_empty() {
            Vec::new()
        } else {
            format!("{}\n", images.join("\n")).into_bytes()
        });
    }
    let mime = match object.get("type") {
        None | Some(Value::Null) => images
            .first()
            .copied()
            .ok_or("no image in host clipboard")?,
        Some(Value::String(mime)) if mime == "image" => images
            .first()
            .copied()
            .ok_or("no image in host clipboard")?,
        Some(Value::String(mime)) if images.contains(&mime.as_str()) => mime.as_str(),
        _ => return Err("requested image type is not in host clipboard".into()),
    };
    let data = clipboard(&["--no-newline", "--type", mime])?;
    if !image_valid(mime, &data) {
        return Err("clipboard changed or returned invalid image bytes".into());
    }
    Ok(data)
}

fn process_request(reader: impl Read) -> Result<Vec<u8>> {
    let mut request = Vec::new();
    BufReader::new(reader)
        .take(4097)
        .read_until(b'\n', &mut request)?;
    if request.len() > 4096 || !request.ends_with(b"\n") {
        return Err("invalid or oversized request".into());
    }
    respond_with(&request, clipboard)
}

fn write_response(mut output: impl Write, result: Result<Vec<u8>>) -> io::Result<()> {
    let (header, data) = match result {
        Ok(data) => (json!({"ok":true,"size":data.len()}), data),
        Err(error) => (
            json!({"ok":false,"size":0,"error":error.to_string()}),
            Vec::new(),
        ),
    };
    serde_json::to_writer(&mut output, &header)?;
    output.write_all(b"\n")?;
    output.write_all(&data)?;
    output.flush()
}

fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        )
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(credentials.uid)
    }
}

fn may_sync_text(types: &str) -> bool {
    !types.lines().any(|mime| mime.starts_with("image/"))
        && types.lines().any(|mime| mime.starts_with("text/plain"))
}

fn sync_text() -> Result<()> {
    // Drain the watcher pipe even for images, without retaining the payload.
    io::copy(&mut io::stdin().lock(), &mut io::sink())?;
    if env::var("CLIPBOARD_STATE")
        .is_ok_and(|state| !matches!(state.as_str(), "data" | "sensitive"))
    {
        return Ok(());
    }
    let types = clipboard(&["--list-types"])?;
    if !may_sync_text(std::str::from_utf8(&types)?) {
        return Ok(());
    }
    let data = clipboard(&["--no-newline", "--type", "text"])?;
    let current = capture("xsel", &["-ob"], None, Duration::from_secs(1));
    if current.as_ref().is_ok_and(|current| current == &data) {
        return Ok(());
    }
    // A second check reduces the chance of overwriting a freshly copied image.
    let types = clipboard(&["--list-types"])?;
    if !may_sync_text(std::str::from_utf8(&types)?) {
        return Ok(());
    }
    capture("xsel", &["-ib"], Some(data), Duration::from_secs(4))?;
    Ok(())
}

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

fn serve(args: &[String]) -> Result<()> {
    let uid = unsafe { libc::getuid() };
    let mut allowed = HashSet::from([uid, 0]);
    let mut sync = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sync-text" => sync = true,
            "--allow-uid" => {
                allowed.insert(args.next().ok_or("missing UID")?.parse::<u32>()?);
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
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
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } < 0 {
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
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = stop as *const () as usize;
        libc::sigemptyset(&mut action.sa_mask);
        if libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut()) < 0
            || libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) < 0
        {
            return Err(io::Error::last_os_error().into());
        }
    }
    let _watcher = if sync {
        Some(ChildGuard(
            Command::new("wl-paste")
                .arg("--watch")
                .arg(&executable)
                .arg("sync-text")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .process_group(0)
                .spawn()?,
            true,
        ))
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
    while RUNNING.load(Ordering::Relaxed) {
        let mut poll = libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // A signal can reach a worker thread instead of the listener thread.
        let ready = unsafe { libc::poll(&mut poll, 1, 1000) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if ready == 0 {
            continue;
        }
        if !RUNNING.load(Ordering::Relaxed) {
            break;
        }
        let (stream, _) = match listener.accept() {
            Ok(client) => client,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
            Err(error) => return Err(error.into()),
        };
        stream.set_read_timeout(Some(Duration::from_secs(12)))?;
        stream.set_write_timeout(Some(Duration::from_secs(12)))?;
        let uid = match peer_uid(&stream) {
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

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = match args.first().map(String::as_str) {
        Some("serve") => serve(&args[1..]),
        Some("sync-text") if args.len() == 1 => sync_text(),
        Some("handle-stdio") if args.len() == 1 => {
            let result = process_request(io::stdin().lock());
            write_response(io::stdout().lock(), result).map_err(Into::into)
        }
        _ => Err("Usage: claude-clipboard-host serve [--sync-text] [--allow-uid UID]".into()),
    };
    if let Err(error) = result {
        eprintln!("claude-clipboard-host: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\xff\x00hello\n";
    fn mock(args: &[&str]) -> Result<Vec<u8>> {
        Ok(if args == ["--list-types"] {
            b"image/png\ntext/plain\n".to_vec()
        } else {
            PNG.to_vec()
        })
    }

    #[test]
    fn should_list_only_supported_image_types() {
        assert_eq!(
            respond_with(br#"{"op":"types"}"#, mock).unwrap(),
            b"image/png\n"
        );
    }

    #[test]
    fn should_return_exact_bytes_for_requested_type() {
        assert_eq!(
            respond_with(br#"{"op":"read","type":"image/png"}"#, mock).unwrap(),
            PNG
        );
    }

    #[test]
    fn should_read_first_offered_image_when_type_is_null() {
        assert_eq!(
            respond_with(br#"{"op":"read","type":null}"#, mock).unwrap(),
            PNG
        );
    }

    #[test]
    fn should_reject_invalid_requests() {
        for request in [
            "bad",
            "[]",
            r#"{"op":"write"}"#,
            r#"{"op":"read","type":42}"#,
            r#"{"op":"read","type":"image/jpeg"}"#,
            r#"{"op":"types","command":"evil"}"#,
        ] {
            assert!(respond_with(request.as_bytes(), mock).is_err(), "{request}");
        }
    }

    #[test]
    fn should_reject_payload_with_wrong_magic() {
        assert!(
            respond_with(br#"{"op":"read","type":"image/png"}"#, |args| Ok(
                if args == ["--list-types"] {
                    b"image/png\n".to_vec()
                } else {
                    b"text".to_vec()
                }
            ))
            .is_err()
        );
    }

    #[test]
    fn should_not_sync_text_when_an_image_is_offered() {
        assert!(!may_sync_text("image/png\ntext/plain\n"));
    }

    #[test]
    fn should_sync_plain_text_offers() {
        assert!(may_sync_text("text/plain;charset=utf-8\ntext/plain\n"));
    }

    #[test]
    fn should_not_sync_text_without_a_plain_text_offer() {
        assert!(!may_sync_text("text/html\n"));
    }

    #[test]
    fn should_frame_response_with_exact_size() {
        let mut output = Vec::new();
        write_response(&mut output, Ok(PNG.to_vec())).unwrap();
        let split = output.iter().position(|byte| *byte == b'\n').unwrap();
        let header: Value = serde_json::from_slice(&output[..split]).unwrap();
        assert_eq!(header["size"], json!(PNG.len()));
        assert_eq!(&output[split + 1..], PNG);
    }

    #[test]
    fn should_time_out_and_reap_a_slow_tool() {
        assert!(capture("sh", &["-c", "sleep 5"], None, Duration::from_millis(50)).is_err());
    }

    #[test]
    fn should_preserve_nuls_and_newlines_in_tool_output() {
        assert_eq!(
            capture("cat", &[], Some(PNG.to_vec()), Duration::from_secs(2)).unwrap(),
            PNG
        );
    }

    #[test]
    fn should_report_peer_uid_of_current_user() {
        let (left, _right) = UnixStream::pair().unwrap();
        assert_eq!(peer_uid(&left).unwrap(), unsafe { libc::getuid() });
    }

    #[test]
    fn should_preserve_binary_payload_across_a_socket() {
        let (mut left, mut right) = UnixStream::pair().unwrap();
        let writer = thread::spawn(move || write_response(&mut right, Ok(PNG.to_vec())).unwrap());
        let mut output = Vec::new();
        left.read_to_end(&mut output).unwrap();
        writer.join().unwrap();
        let split = output.iter().position(|byte| *byte == b'\n').unwrap();
        assert_eq!(&output[split + 1..], PNG);
    }
}
