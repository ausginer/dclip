use crate::support::Scratch;
use std::{fs, os::unix::net::UnixListener};

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
    for signal in ["TERM", "INT"] {
        let scratch = Scratch::new();
        scratch.clipboard(b"", b"");
        let mut server = scratch.serve(&[]);
        server.signal(signal);
        assert_eq!(server.wait().code(), Some(0), "SIG{signal}");
        assert!(!scratch.socket().exists(), "SIG{signal} left the socket");
    }
}
