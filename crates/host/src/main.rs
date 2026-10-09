//! `dclip`: serves the host's clipboard images to a
//! devcontainer over a Unix socket, and optionally mirrors plain text into X11.

mod cli;
mod clipboard;
mod image;
mod process;
mod protocol;
mod server;
mod sync;
mod sys;

use std::env;

/// Every error ends as text, so it is a boxed message.
pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    let result = cli::parse(env::args_os().skip(1)).and_then(|command| match command {
        cli::Command::Serve(options) => server::serve(&options),
        cli::Command::SyncText => sync::run(),
    });
    if let Err(error) = result {
        eprintln!("dclip: {error}");
        std::process::exit(1);
    }
}
