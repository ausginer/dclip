use crate::support::Scratch;

#[test]
fn should_print_usage_and_exit_1_for_an_unpublished_invocation() {
    let scratch = Scratch::new();
    for args in [&[][..], &["bogus"], &["sync-text", "extra"]] {
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
fn should_exit_1_for_malformed_serve_arguments() {
    let scratch = Scratch::new();
    for args in [
        &["serve", "--bogus"][..],
        &["serve", "--allow-uid", "alice"],
        &["serve", "--allow-uid"],
    ] {
        let output = scratch.run(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(!scratch.socket().exists(), "{args:?} bound the socket");
    }
}
