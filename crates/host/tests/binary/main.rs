//! The binary-level layer: each test runs the built `dclip`
//! as a user would, with stand-in tools in place of Wayland and X11.

mod cli;
mod protocol;
mod server;
mod support;
mod sync;
