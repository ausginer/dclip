use crate::support::Scratch;
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

/// An argument that is not UTF-8.
fn not_utf8() -> &'static OsStr {
    OsStr::from_bytes(b"\xff")
}

#[test]
fn should_print_usage_and_exit_1_for_an_unpublished_invocation() {
    let scratch = Scratch::new();
    for args in [
        &[][..],
        &["bogus"].map(OsStr::new),
        &["sync-text", "extra"].map(OsStr::new),
        &["handle-stdio"].map(OsStr::new),
        &[not_utf8()],
        &["sync-text", "--"].map(OsStr::new),
    ] {
        let output = scratch.run(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).starts_with("dclip: Usage: dclip serve"),
            "{args:?}: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn should_exit_1_naming_the_argument_at_fault_for_malformed_serve_arguments() {
    let scratch = Scratch::new();
    for (args, message) in [
        (
            &["serve", "--bogus"].map(OsStr::new)[..],
            "unknown argument: --bogus",
        ),
        (
            &["serve", "--allow-uid", "alice"].map(OsStr::new),
            "not a UID: alice",
        ),
        (&["serve", "--allow-uid"].map(OsStr::new), "missing UID"),
        (
            &[OsStr::new("serve"), not_utf8()],
            "unknown argument: \u{FFFD}",
        ),
        (
            &[OsStr::new("serve"), OsStr::new("--allow-uid"), not_utf8()],
            "not a UID: \u{FFFD}",
        ),
        (&["serve", "-x"].map(OsStr::new), "unknown argument: -x"),
        (
            &["serve", "--sync-text=yes"].map(OsStr::new),
            "unknown argument: --sync-text=yes",
        ),
        (
            &["serve", "--", "extra"].map(OsStr::new),
            "unknown argument: extra",
        ),
    ] {
        let output = scratch.run(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("dclip: {message}\n"),
            "{args:?}"
        );
        assert!(!scratch.socket().exists(), "{args:?} bound the socket");
    }
}
