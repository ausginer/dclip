//! `claude-clipboard-host`: serves the host's clipboard images to a
//! devcontainer over a Unix socket, and optionally mirrors plain text into X11.

mod cli;
mod clipboard;
mod image;
mod process;
mod protocol;
mod server;
mod sync;
mod sys;

use std::{env, io};

/// Every error ends as text, so it is a boxed message.
pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = cli::parse(&args).and_then(|command| match command {
        cli::Command::Serve(options) => server::serve(&options),
        cli::Command::SyncText => sync::run(),
        cli::Command::HandleStdio => {
            let result = protocol::process_request(io::stdin().lock());
            protocol::write_response(io::stdout().lock(), result).map_err(Into::into)
        }
    });
    if let Err(error) = result {
        eprintln!("claude-clipboard-host: {error}");
        std::process::exit(1);
    }
}
