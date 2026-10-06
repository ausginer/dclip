use crate::support::{Scratch, await_pid, running};
use std::{fs, io::Write, os::unix::net::UnixListener};

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
        assert!(!scratch.socket().exists(), "SIG{signal} left the socket");
    }
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
