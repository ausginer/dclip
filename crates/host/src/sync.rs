//! Text sync: run by `wl-paste --watch` on each clipboard change, it mirrors
//! plain text from Wayland into the X11 CLIPBOARD selection, and leaves any
//! offer that contains an image alone.

use crate::{Result, clipboard, clipboard::Query};
use std::{env, io};

/// The `sync-text` subcommand.
pub(crate) fn run() -> Result<()> {
    // Drain the watcher pipe even for images, without retaining the payload, so
    // `wl-paste --watch` never blocks writing to it.
    io::copy(&mut io::stdin().lock(), &mut io::sink())?;
    sync_text(
        env::var("CLIPBOARD_STATE").ok().as_deref(),
        clipboard::wayland,
        clipboard::x11_text,
        clipboard::set_x11_text,
    )
}

/// Whether an offer listed as `types` is plain text with no image in it.
fn may_sync_text(types: &[u8]) -> bool {
    !clipboard::lines(types).any(|mime| mime.starts_with(b"image/"))
        && clipboard::lines(types).any(|mime| mime.starts_with(b"text/plain"))
}

/// The sync sequence, with the tools passed in. `state` is the watcher's
/// `CLIPBOARD_STATE`, if it set one.
pub(crate) fn sync_text(
    state: Option<&str>,
    mut wayland: impl FnMut(Query) -> Result<Vec<u8>>,
    x11_text: impl FnOnce() -> Result<Vec<u8>>,
    set_x11_text: impl FnOnce(Vec<u8>) -> Result<()>,
) -> Result<()> {
    if state.is_some_and(|state| !matches!(state, "data" | "sensitive")) {
        return Ok(());
    }
    if !may_sync_text(&wayland(Query::Types)?) {
        return Ok(());
    }
    let text = wayland(Query::Text)?;
    // Writing X11 makes XWayland update the Wayland clipboard, which runs this
    // again; stopping on equal text is what ends that loop. A failed read
    // counts as different.
    if x11_text().is_ok_and(|current| current == text) {
        return Ok(());
    }
    // A second listing narrows the window in which a freshly copied image
    // could be overwritten by text. Only the compositor could close it.
    if !may_sync_text(&wayland(Query::Types)?) {
        return Ok(());
    }
    set_x11_text(text)
}

#[cfg(test)]
mod tests;
