//! The command line: what the binary was asked to do.

use crate::Result;

pub(crate) const USAGE: &str = "Usage: claude-clipboard-host serve [--sync-text] [--allow-uid UID]";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Serve(ServeOptions),
    SyncText,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ServeOptions {
    /// Run the text-sync watcher beside the server.
    pub(crate) sync_text: bool,
    /// Peer UIDs allowed besides the serving UID and root.
    pub(crate) allow_uids: Vec<u32>,
}

/// Parses the arguments after the program name.
pub(crate) fn parse(args: &[String]) -> Result<Command> {
    match args {
        [command, options @ ..] if command == "serve" => {
            let mut parsed = ServeOptions::default();
            let mut options = options.iter();
            while let Some(option) = options.next() {
                match option.as_str() {
                    "--sync-text" => parsed.sync_text = true,
                    "--allow-uid" => {
                        let uid = options.next().ok_or("missing UID")?;
                        parsed
                            .allow_uids
                            .push(uid.parse().map_err(|_| format!("not a UID: {uid}"))?);
                    }
                    _ => return Err(format!("unknown argument: {option}").into()),
                }
            }
            Ok(Command::Serve(parsed))
        }
        [command] if command == "sync-text" => Ok(Command::SyncText),
        _ => Err(USAGE.into()),
    }
}
