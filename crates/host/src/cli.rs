//! The command line: what the binary was asked to do.

use crate::Result;
use lexopt::Arg;
use std::ffi::{OsStr, OsString};

#[cfg(test)]
mod tests;

pub(crate) const USAGE: &str = "Usage: dclip serve [--sync-text] [--allow-uid UID]";

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
///
/// The subcommand is the first argument as given, so `--` never selects one
/// and `sync-text --` is an argument to `sync-text`. Only `serve`'s options go
/// through `lexopt`, which accepts `--allow-uid=UID` and ends them at `--`.
pub(crate) fn parse(args: impl IntoIterator<Item = impl Into<OsString>>) -> Result<Command> {
    let mut args = args.into_iter().map(Into::into);
    match args.next().as_deref().and_then(OsStr::to_str) {
        Some("serve") => {
            let mut options = ServeOptions::default();
            let mut parser = lexopt::Parser::from_args(args);
            // `next` fails only on a value attached to the previous option and
            // left unread, and every arm below reads that value or refuses.
            while let Some(arg) = parser.next()? {
                match arg {
                    Arg::Long("sync-text") => {
                        if let Some(value) = parser.optional_value() {
                            return Err(format!(
                                "unknown argument: --sync-text={}",
                                value.display()
                            )
                            .into());
                        }
                        options.sync_text = true;
                    }
                    Arg::Long("allow-uid") => {
                        let uid = parser.value().map_err(|_| "missing UID")?;
                        options.allow_uids.push(
                            uid.to_str()
                                .and_then(|uid| uid.parse().ok())
                                .ok_or_else(|| format!("not a UID: {}", uid.display()))?,
                        );
                    }
                    Arg::Long(option) => return Err(format!("unknown argument: --{option}").into()),
                    Arg::Short(option) => return Err(format!("unknown argument: -{option}").into()),
                    Arg::Value(operand) => {
                        return Err(format!("unknown argument: {}", operand.display()).into());
                    }
                }
            }
            Ok(Command::Serve(options))
        }
        Some("sync-text") if args.next().is_none() => Ok(Command::SyncText),
        _ => Err(USAGE.into()),
    }
}
