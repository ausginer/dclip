use super::*;

fn serve(sync_text: bool, allow_uids: &[u32]) -> Command {
    Command::Serve(ServeOptions {
        sync_text,
        allow_uids: allow_uids.to_vec(),
    })
}

#[test]
fn should_accept_the_getopt_spellings_of_the_published_invocations() {
    for (args, command) in [
        (&["serve"][..], serve(false, &[])),
        (&["serve", "--"], serve(false, &[])),
        (&["serve", "--allow-uid=1001"], serve(false, &[1001])),
        (&["serve", "--sync-text", "--sync-text"], serve(true, &[])),
        (
            &["serve", "--allow-uid", "1001", "--allow-uid=1002"],
            serve(false, &[1001, 1002]),
        ),
        (
            &["serve", "--sync-text", "--allow-uid", "1001"],
            serve(true, &[1001]),
        ),
        (
            &["serve", "--allow-uid", "1001", "--sync-text", "--"],
            serve(true, &[1001]),
        ),
        (&["sync-text"], Command::SyncText),
    ] {
        let parsed = parse(args.iter().copied());
        assert_eq!(parsed.ok(), Some(command), "{args:?}");
    }
}
