use crate::support::Scratch;

#[test]
fn should_print_usage_and_exit_1_for_an_unpublished_invocation() {
    let scratch = Scratch::new();
    for args in [
        &[][..],
        &["bogus"],
        &["sync-text", "extra"],
        &["handle-stdio"],
    ] {
        let output = scratch.run(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .starts_with("claude-clipboard-host: Usage: claude-clipboard-host serve"),
            "{args:?}: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn should_exit_1_naming_the_argument_at_fault_for_malformed_serve_arguments() {
    let scratch = Scratch::new();
    for (args, message) in [
        (&["serve", "--bogus"][..], "unknown argument: --bogus"),
        (&["serve", "--allow-uid", "alice"], "not a UID: alice"),
        (&["serve", "--allow-uid"], "missing UID"),
    ] {
        let output = scratch.run(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("claude-clipboard-host: {message}\n"),
            "{args:?}"
        );
        assert!(!scratch.socket().exists(), "{args:?} bound the socket");
    }
}
