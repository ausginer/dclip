use crate::support::{PATIENCE, Scratch, Socket, await_pid, read_framed, read_response, running};
use std::{
    fs,
    io::Write,
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[test]
fn should_refuse_a_second_instance_while_the_first_holds_the_lock() {
    let scratch = Scratch::new();
    scratch.clipboard(b"", b"");
    let _first = scratch.serve(&[]);
    let output = scratch.run(&["serve"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("already running"),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn should_replace_a_stale_socket_owned_by_the_user() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", b"");
    drop(UnixListener::bind(scratch.socket()).unwrap());
    assert!(scratch.socket().exists());
    let server = scratch.serve(&[]);
    let response = server.request(b"{\"op\":\"types\"}\n");
    assert_eq!(response.header["ok"], true);
}

#[test]
fn should_refuse_and_keep_a_regular_file_at_the_socket_path() {
    let scratch = Scratch::new();
    fs::write(scratch.socket(), b"keep").unwrap();
    let output = scratch.run(&["serve"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read(scratch.socket()).unwrap(), b"keep");
}

#[test]
fn should_remove_the_socket_and_exit_0_on_a_termination_signal() {
    for signal in ["TERM", "INT", "HUP"] {
        let scratch = Scratch::new();
        scratch.clipboard(b"", b"");
        let mut server = scratch.serve(&[]);
        server.signal(signal);
        assert_eq!(server.wait().code(), Some(0), "SIG{signal}");
        assert_eq!(server.stderr(), "", "SIG{signal}");
        assert!(!scratch.socket().exists(), "SIG{signal} left the socket");
    }
}

#[test]
fn should_keep_serving_after_peers_that_left_before_they_were_accepted() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", b"");
    let server = scratch.serve(&[]);
    // Stopped, `serve` accepts nothing, so every peer below has gone by the
    // time its connection is accepted. There are fifteen, so that with the
    // request after them no connection meets the limit of sixteen.
    server.signal("STOP");
    for sent in [&b""[..], b"{\"op\":", b"{\"op\":\"types\"}\n"] {
        for _ in 0..5 {
            server.connect().write_all(sent).unwrap();
        }
    }
    server.signal("CONT");
    let response = server.request(b"{\"op\":\"types\"}\n");
    assert_eq!(response.header["ok"], true, "{:?}", response.header);
}

#[test]
fn should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal() {
    let scratch = Scratch::new();
    scratch.write("types", b"image/png\n");
    scratch.write("payload", b"\x89PNG\r\n\x1a\n");
    // The read takes a second, well inside its deadline, so only a server that
    // waits for its workers sees it end.
    scratch.tool(
        "wl-paste",
        r#"case "$1" in
  --list-types) exec cat "$DIR/types" ;;
esac
echo $$ > "$DIR/reading.tmp" && mv "$DIR/reading.tmp" "$DIR/reading"
sleep 1
exec cat "$DIR/payload""#,
    );
    let mut server = scratch.serve(&[]);
    let mut stream = server.connect();
    stream
        .write_all(b"{\"op\":\"read\",\"type\":\"image/png\"}\n")
        .unwrap();
    let tool = await_pid(&scratch.path("reading"));
    server.signal("TERM");
    assert_eq!(server.wait().code(), Some(0));
    assert!(!running(tool), "serve exited before reaping its tool");
}

#[test]
fn should_stop_advertising_and_keep_the_lock_while_draining_on_a_signal() {
    let scratch = Scratch::new();
    scratch.write("types", b"image/png\n");
    scratch.write("payload", b"\x89PNG\r\n\x1a\n");
    // The read lasts until the test creates `release`, so the drain lasts
    // exactly as long as the assertions need, within the tool's 4 s deadline.
    scratch.tool(
        "wl-paste",
        r#"case "$1" in
  --list-types) exec cat "$DIR/types" ;;
esac
echo $$ > "$DIR/reading.tmp" && mv "$DIR/reading.tmp" "$DIR/reading"
while [ ! -e "$DIR/release" ]; do sleep 0.01; done
exec cat "$DIR/payload""#,
    );
    let mut server = scratch.serve(&[]);
    let mut stream = server.connect();
    stream
        .write_all(b"{\"op\":\"read\",\"type\":\"image/png\"}\n")
        .unwrap();
    let tool = await_pid(&scratch.path("reading"));
    let signalled = Instant::now();
    server.signal("TERM");
    while scratch.socket().exists() {
        assert!(
            signalled.elapsed() < Duration::from_secs(1),
            "the socket outlived the signal"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        UnixStream::connect(scratch.socket()).is_err(),
        "a connect succeeded during the drain"
    );
    let successor = scratch.run(&["serve"]);
    assert_eq!(successor.status.code(), Some(1), "{successor:?}");
    assert!(
        String::from_utf8_lossy(&successor.stderr).contains("already running"),
        "{:?}",
        String::from_utf8_lossy(&successor.stderr)
    );
    assert!(server.alive(), "the drain ended before its tool finished");
    scratch.write("release", b"");
    assert_eq!(server.wait().code(), Some(0));
    assert!(!running(tool), "serve exited before reaping its tool");
}

#[test]
fn should_keep_a_signal_ignored_when_serve_inherits_it_ignored() {
    // Each row stops `serve` with a handled signal other than the ignored one.
    for (ignored, stop) in [("TERM", "INT"), ("INT", "HUP"), ("HUP", "TERM")] {
        let scratch = Scratch::new();
        scratch.clipboard(b"image/png\n", b"");
        let mut server = scratch.serve_ignoring(ignored);
        server.signal(ignored);
        // Long enough for a bridge that handles the signal to have exited.
        thread::sleep(Duration::from_millis(200));
        assert!(server.alive(), "serve stopped on an ignored SIG{ignored}");
        let response = server.request(b"{\"op\":\"types\"}\n");
        assert_eq!(response.header["ok"], true, "SIG{ignored}");
        server.signal(stop);
        assert_eq!(
            server.wait().code(),
            Some(0),
            "SIG{stop} after SIG{ignored}"
        );
    }
}

#[test]
fn should_unadvertise_at_once_and_turn_away_an_idle_peer_on_a_signal() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", b"");
    let mut server = scratch.serve(&[]);
    let idle = server.connect();
    // Connections are accepted in order, so once this one is answered the idle
    // one has been accepted and its worker is waiting for a request.
    assert_eq!(server.request(b"{\"op\":\"types\"}\n").header["ok"], true);
    let signalled = Instant::now();
    server.signal("TERM");
    while scratch.socket().exists() {
        assert!(
            signalled.elapsed() < Duration::from_secs(1),
            "the socket outlived the signal"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let response = read_response(idle);
    assert_eq!(
        response.header,
        serde_json::json!({"error": "bridge is shutting down", "ok": false, "size": 0})
    );
    assert_eq!(server.wait().code(), Some(0));
    // A request phase is 12 s; a drain that waited one out would take that.
    assert!(
        signalled.elapsed() < Duration::from_secs(2),
        "{:?}",
        signalled.elapsed()
    );
}

#[test]
fn should_refuse_a_connection_over_the_limit_and_keep_serving() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\n", b"");
    let server = scratch.serve(&[]);
    // Each holds a place by sending nothing; they are accepted in order, so
    // the seventeenth finds every place taken.
    let idle = (0..16).map(|_| server.connect()).collect::<Vec<_>>();
    assert_eq!(
        read_response(server.connect()).header,
        serde_json::json!({
            "error": "too many concurrent clipboard requests",
            "ok": false,
            "size": 0
        })
    );
    drop(idle);
    // A place comes free once its worker has seen its peer go, which the
    // test cannot observe, so it asks until a place has, and only the limit's
    // refusal is another attempt. A refused connection is closed with its
    // request unread, so the send can fail after the refusal has already
    // arrived; the response is read either way, and by its framing, because
    // reading on to the end of the stream could meet the reset the unread
    // request causes. Any other failure fails the test.
    let deadline = Instant::now() + PATIENCE;
    loop {
        let mut stream = server.connect();
        let sent = stream.write_all(b"{\"op\":\"types\"}\n");
        let response = read_framed(&stream)
            .unwrap_or_else(|error| panic!("{error}, after the send gave {sent:?}"));
        if response.header["ok"] == true {
            break;
        }
        assert_eq!(
            response.header["error"], "too many concurrent clipboard requests",
            "{:?}",
            response.header
        );
        assert!(Instant::now() < deadline, "no place came free");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn should_bind_the_default_socket_in_a_directory_it_creates_at_0755() {
    let scratch = Scratch::new();
    scratch.clipboard(b"", b"");
    let home = scratch.path("home");
    let directory = home.join(".local/share/dclip");
    let mut server = scratch.serve_at_default(&home);
    assert!(
        directory.join("clipboard.sock").exists(),
        "no default socket"
    );
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o755
    );
    server.signal("TERM");
    assert_eq!(server.wait().code(), Some(0));
    assert!(!directory.join("clipboard.sock").exists());
}

#[test]
fn should_exit_1_naming_both_variables_without_home_or_a_socket_override() {
    let scratch = Scratch::new();
    for home in [None, Some(Path::new(""))] {
        let output = scratch.run_with(Socket::Default(home), &["serve"]);
        assert_eq!(output.status.code(), Some(1), "HOME {home:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "dclip: neither DCLIP_SOCKET nor HOME is set\n",
            "HOME {home:?}"
        );
    }
}
