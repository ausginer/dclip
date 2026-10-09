//! Drives the built binary through its real surface: arguments, exit status,
//! stderr, the socket and signals. Stand-in `wl-paste` and `xsel` scripts sit
//! first on the child's `PATH`, in place of a Wayland session.
// Test support: `allow-unwrap-in-tests` covers `#[test]` functions only, and a
// panic here is a failed test like any other.
#![allow(clippy::unwrap_used)]

use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
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

    /// The binary with `args`, the stand-ins first on its `PATH` and this
    /// scratch directory's socket. It starts through GNU `env`, so SIGTERM,
    /// SIGINT and SIGHUP begin at their default dispositions whatever the test
    /// runner inherited, apart from `ignored`, which begins ignored. The bridge
    /// keeps a disposition it inherited as ignored, so without this a runner
    /// under `nohup` would fail a correct bridge. A POSIX shell cannot restore
    /// an inherited ignore; `env` can from coreutils 8.31 on.
    fn command(&self, ignored: Option<&str>, args: &[&str]) -> Command {
        let restored = ["TERM", "INT", "HUP"]
            .into_iter()
            .filter(|signal| Some(*signal) != ignored)
            .collect::<Vec<_>>()
            .join(",");
        let mut command = Command::new("env");
        command.arg(format!("--default-signal={restored}"));
        if let Some(signal) = ignored {
            command.arg(format!("--ignore-signal={signal}"));
        }
        let mut path = self.dir.join("bin").into_os_string();
        if let Some(inherited) = std::env::var_os("PATH") {
            path.push(":");
            path.push(inherited);
        }
        command
            .arg(BINARY)
            .args(args)
            .env("PATH", path)
            .env("CLAUDE_CLIPBOARD_SOCKET", self.socket())
            .env_remove("CLIPBOARD_STATE");
        command
    }

    /// Runs the binary to completion. One that is still running after
    /// [`PATIENCE`] — a `serve` that should have refused, say — is killed and
    /// fails the test rather than hanging it.
    pub fn run(&self, args: &[&str]) -> Output {
        let child = self
            .command(None, args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut running = Running(child);
        let deadline = Instant::now() + PATIENCE;
        while running.0.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "{args:?} did not exit");
            thread::sleep(Duration::from_millis(10));
        }
        let mut output = Output {
            status: running.0.wait().unwrap(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        if let Some(mut stdout) = running.0.stdout.take() {
            stdout.read_to_end(&mut output.stdout).unwrap();
        }
        if let Some(mut stderr) = running.0.stderr.take() {
            stderr.read_to_end(&mut output.stderr).unwrap();
        }
        output
    }

    /// Starts `serve` and returns once it has announced itself, which it does
    /// after the socket is bound and its signal handlers are installed.
    pub fn serve(&self, args: &[&str]) -> Server {
        self.start(self.command(None, &[&["serve"], args].concat()))
    }

    /// Starts `serve` as [`Scratch::serve`] does, but with `signal` ignored
    /// when it starts, the way `nohup` starts a program with SIGHUP ignored.
    pub fn serve_ignoring(&self, signal: &str) -> Server {
        self.start(self.command(Some(signal), &["serve"]))
    }

    fn start(&self, mut command: Command) -> Server {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut server = Server {
            child: Running(child),
            socket: self.socket(),
        };
        // Read on a thread of its own, so that a `serve` that neither
        // announces itself nor exits fails the test instead of hanging it.
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut banner = String::new();
            let _ = BufReader::new(stdout).read_line(&mut banner);
            let _ = sender.send(banner);
        });
        let banner = receiver
            .recv_timeout(PATIENCE)
            .expect("serve did not announce itself");
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

/// A running `serve`, killed and reaped on drop.
pub struct Server {
    child: Running,
    socket: PathBuf,
}

/// A child killed and reaped on drop, so a failed assertion never leaves one
/// behind.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct Response {
    pub header: Value,
    pub payload: Vec<u8>,
}

impl Server {
    pub fn pid(&self) -> u32 {
        self.child.0.id()
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
            if let Some(status) = self.child.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "serve did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Whether the process has yet to exit.
    pub fn alive(&mut self) -> bool {
        self.child.0.try_wait().unwrap().is_none()
    }

    /// Whatever the process has written to stderr. Only after it has exited.
    pub fn stderr(&mut self) -> String {
        let mut text = String::new();
        if let Ok(Some(_)) = self.child.0.try_wait()
            && let Some(mut stderr) = self.child.0.stderr.take()
        {
            stderr.read_to_string(&mut text).unwrap();
        }
        text
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

/// Whether process `pid` is running: it exists and is not a zombie. A process
/// that has died but whose new parent has not reaped it counts as gone.
pub fn running(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| !rest.starts_with('Z'))
    })
}

/// Waits for `path` to exist, for a file a stand-in writes asynchronously.
pub fn await_path(path: &Path) {
    let deadline = Instant::now() + PATIENCE;
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "{} never appeared",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

/// The PID a stand-in wrote to `path`, once it has.
pub fn await_pid(path: &Path) -> u32 {
    await_path(path);
    let deadline = Instant::now() + PATIENCE;
    loop {
        if let Ok(pid) = fs::read_to_string(path).unwrap().trim().parse() {
            return pid;
        }
        assert!(Instant::now() < deadline, "{} holds no PID", path.display());
        thread::sleep(Duration::from_millis(10));
    }
}
