use crate::support::{PATIENCE, Scratch, await_pid, running, signal_pid};
use std::{
    fs, thread,
    time::{Duration, Instant},
};

/// A stand-in `xsel` whose selection is the file `x11`: `-ob` prints it,
/// failing when it is absent, and `-ib` replaces it with stdin.
fn x11(scratch: &Scratch) {
    scratch.tool(
        "xsel",
        r#"case "$1" in
  -ob) exec cat "$DIR/x11" 2>/dev/null ;;
  -ib) exec cat > "$DIR/x11" ;;
esac
exit 2"#,
    );
}

#[test]
fn should_write_offered_plain_text_to_x11() {
    let scratch = Scratch::new();
    scratch.clipboard(b"text/plain;charset=utf-8\ntext/plain\n", b"copied\ntext");
    x11(&scratch);
    let output = scratch.run(&["sync-text"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(fs::read(scratch.path("x11")).unwrap(), b"copied\ntext");
}

#[test]
fn should_not_sync_text_when_an_image_is_offered() {
    let scratch = Scratch::new();
    scratch.clipboard(b"image/png\ntext/plain\n", b"copied");
    x11(&scratch);
    let output = scratch.run(&["sync-text"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!scratch.path("x11").exists());
}

#[test]
fn should_end_the_watcher_when_serve_is_killed() {
    let scratch = Scratch::new();
    scratch.tool(
        "wl-paste",
        r#"[ "$1" = --watch ] || exit 2
echo $$ > "$DIR/watcher.tmp" && mv "$DIR/watcher.tmp" "$DIR/watcher"
exec sleep 300"#,
    );
    let mut server = scratch.serve(&["--sync-text"]);
    let watcher = await_pid(&scratch.path("watcher"));
    server.signal("KILL");
    server.wait();
    let deadline = Instant::now() + PATIENCE;
    while running(watcher) {
        if Instant::now() >= deadline {
            signal_pid(watcher, "KILL");
            panic!("the watcher outlived serve");
        }
        thread::sleep(Duration::from_millis(10));
    }
}
