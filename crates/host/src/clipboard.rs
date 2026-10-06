//! What the bridge asks of `wl-paste` and `xsel`.

use crate::{
    Result,
    image::Format,
    process::{Tool, capture},
};
use std::{
    io,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

/// One question for the Wayland clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Query {
    /// The offered MIME types, one per line.
    Types,
    Image(Format),
    /// Plain text, as `wl-paste` chooses to convert it.
    Text,
}

/// The lines of a type listing, as `str::lines` splits them but over bytes: a
/// listing holds whatever the source application offered, and need not be
/// UTF-8. Every supported type is ASCII, so byte comparison loses nothing.
pub(crate) fn lines(listing: &[u8]) -> impl Iterator<Item = &[u8]> {
    listing
        .strip_suffix(b"\n")
        .unwrap_or(listing)
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
}

const WAYLAND_DEADLINE: Duration = Duration::from_secs(4);

pub(crate) fn wayland(query: Query) -> Result<Vec<u8>> {
    let args: &[&str] = match query {
        Query::Types => &["--list-types"],
        Query::Image(format) => &["--no-newline", "--type", format.mime()],
        Query::Text => &["--no-newline", "--type", "text"],
    };
    capture("wl-paste", args, None, WAYLAND_DEADLINE)
}

/// The X11 CLIPBOARD selection as text.
pub(crate) fn x11_text() -> Result<Vec<u8>> {
    capture("xsel", &["-ob"], None, Duration::from_secs(1))
}

/// Makes `text` the X11 CLIPBOARD selection. `xsel` forks a daemon to own the
/// selection, and that daemon outlives the capture by design.
pub(crate) fn set_x11_text(text: Vec<u8>) -> Result<()> {
    capture("xsel", &["-ib"], Some(text), Duration::from_secs(4))?;
    Ok(())
}

/// Starts `wl-paste --watch`, which runs `executable sync-text` on every
/// clipboard change.
pub(crate) fn watch(executable: &Path) -> io::Result<Tool> {
    Tool::spawn(
        Command::new("wl-paste")
            .arg("--watch")
            .arg(executable)
            .arg("sync-text")
            .stdin(Stdio::null())
            .stdout(Stdio::null()),
    )
}
