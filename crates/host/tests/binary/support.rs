//! Drives the built binary through its real surface: arguments, exit status,
//! stderr, the socket and signals. Stand-in `wl-paste` and `xsel` scripts sit
//! first on the child's `PATH`, in place of a Wayland session.

use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

pub const BINARY: &str = env!("CARGO_BIN_EXE_claude-clipboard-host");

/// How long a test waits for the binary to reach a state it is expected to
/// reach promptly. Generous, because a loaded machine runs tests in parallel.
pub const PATIENCE: Duration = Duration::from_secs(10);

/// A directory of the test's own, removed on drop. It lives under the system
/// temporary directory rather than the target directory, so the socket path
/// stays inside the 107-byte `sun_path` limit wherever the checkout is.
pub struct Scratch {
    pub dir: PathBuf,
}

impl Scratch {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("bin")).unwrap();
        let scratch = Self { dir };
        assert!(
            scratch.socket().as_os_str().len() < 108,
            "socket path too long"
        );
        scratch
    }

    pub fn socket(&self) -> PathBuf {
        self.dir.join("s.sock")
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    /// Places a `/bin/sh` script named `name` first on the binary's `PATH`.
    /// `$DIR` in the body is the scratch directory.
    pub fn tool(&self, name: &str, body: &str) {
        let path = self.dir.join("bin").join(name);
        let script = format!("#!/bin/sh\nDIR='{}'\n{body}\n", self.dir.display());
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A stand-in `wl-paste` that lists the file `types` and reads the file
    /// `payload`, whatever type is asked for.
    pub fn clipboard(&self, types: &[u8], payload: &[u8]) {
        self.write("types", types);
        self.write("payload", payload);
        self.tool(
            "wl-paste",
            r#"case "$1" in
  --list-types) exec cat "$DIR/types" ;;
  *) exec cat "$DIR/payload" ;;
esac"#,
        );
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut path = self.dir.join("bin").into_os_string();
        if let Some(inherited) = std::env::var_os("PATH") {
            path.push(":");
            path.push(inherited);
        }
        let mut command = Command::new(BINARY);
        command
            .args(args)
            .env("PATH", path)
            .env("CLAUDE_CLIPBOARD_SOCKET", self.socket())
            .env_remove("CLIPBOARD_STATE");
        command
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.command(args).stdin(Stdio::null()).output().unwrap()
    }

    /// Starts `serve` and returns once it has announced itself, which it does
    /// after the socket is bound and its signal handlers are installed.
    pub fn serve(&self, args: &[&str]) -> Server {
        let mut command = self.command(&[&["serve"], args].concat());
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut server = Server {
            child,
            socket: self.socket(),
        };
        let mut banner = String::new();
        BufReader::new(stdout).read_line(&mut banner).unwrap();
        assert!(
            banner.starts_with("Clipboard bridge: "),
            "serve did not start: {banner:?}, {:?}",
            server.stderr()
        );
        server
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A running `serve`, killed and reaped on drop, so a failed assertion never
/// leaves one behind.
pub struct Server {
    child: Child,
    socket: PathBuf,
}

pub struct Response {
    pub header: Value,
    pub payload: Vec<u8>,
}

impl Server {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn connect(&self) -> UnixStream {
        UnixStream::connect(&self.socket).unwrap()
    }

    /// Sends `line` as written and parses the response without the host's own
    /// framing code: the header is the bytes before the first newline, the
    /// payload everything after it.
    pub fn request(&self, line: &[u8]) -> Response {
        let mut stream = self.connect();
        stream.write_all(line).unwrap();
        read_response(stream)
    }

    pub fn signal(&self, signal: &str) {
        signal_pid(self.pid(), signal);
    }

    /// Waits for the process to exit by itself.
    pub fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "serve did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Whatever the process has written to stderr. Only after it has exited.
    pub fn stderr(&mut self) -> String {
        let mut text = String::new();
        if let Some(mut stderr) = self.child.stderr.take() {
            if self.child.try_wait().ok().flatten().is_some() {
                stderr.read_to_string(&mut text).unwrap();
            }
        }
        text
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn read_response(mut stream: impl Read) -> Response {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    let split = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("response has no header line");
    Response {
        header: serde_json::from_slice(&bytes[..split]).unwrap(),
        payload: bytes[split + 1..].to_vec(),
    }
}

pub fn signal_pid(pid: u32, signal: &str) {
    let status = Command::new("/bin/sh")
        .args(["-c", &format!("kill -{signal} {pid}")])
        .status()
        .unwrap();
    assert!(status.success(), "kill -{signal} {pid}");
}
