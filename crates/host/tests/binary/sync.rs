use crate::support::Scratch;
use std::fs;

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
