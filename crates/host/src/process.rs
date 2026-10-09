//! Running an external tool under a deadline, and making sure it is reaped.

use crate::{
    Result,
    sys::{self, Interest, Poll},
};
use std::{
    io::{self, Read, Write},
    os::{fd::AsFd, unix::process::CommandExt},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

/// The most a tool may print, which is also the largest image the bridge serves.
pub(crate) const LIMIT: usize = 64 * 1024 * 1024;

/// Starts `command` as the leader of a process group of its own, so that the
/// bridge can kill whatever it started without touching anything else — in
/// particular, killing one tool's group never takes the daemon `xsel -i`
/// forks to own the X11 selection.
pub(crate) fn spawn(command: &mut Command) -> io::Result<Child> {
    command.process_group(0).spawn()
}

/// A spawned tool. While it is unreaped its PID, and so its process group, is
/// still its own, and dropping it kills the group and reaps the child, on
/// every path out, panics included. Once reaped, the PID may belong to someone
/// else, so the tool is never signalled again and only its status remains.
pub(crate) struct Tool(State);

/// Kept apart from [`Tool`] so that moving to `Reaped` drops the `Child`
/// handle, which signals nothing, without running `Tool`'s `Drop`.
enum State {
    Unreaped(Child),
    Reaped(ExitStatus),
}

impl Tool {
    pub(crate) fn new(child: Child) -> Self {
        Self(State::Unreaped(child))
    }

    /// The exit status, reaping the child if it has exited.
    fn try_reap(&mut self) -> io::Result<Option<ExitStatus>> {
        match &mut self.0 {
            State::Unreaped(child) => {
                let status = child.try_wait()?;
                if let Some(status) = status {
                    self.0 = State::Reaped(status);
                }
                Ok(status)
            }
            State::Reaped(status) => Ok(Some(*status)),
        }
    }
}

impl Drop for Tool {
    fn drop(&mut self) {
        match &mut self.0 {
            State::Unreaped(child) => {
                sys::kill_group(child.id());
                let _ = child.wait();
            }
            State::Reaped(_) => {}
        }
    }
}

/// Runs `program`, feeds it `input`, and returns what it prints, failing if it
/// exits unsuccessfully, prints more than [`LIMIT`] or is not done — input
/// taken, output closed and process exited — within `deadline`.
///
/// The thread sleeps in `poll` until the tool's stdin is writable, its stdout
/// is readable, it has exited, or the deadline passes, so the time a capture
/// takes is the tool's own plus the cost of moving its bytes. Nothing outlives
/// the capture: stdin is written from this loop, and its end of the pipe is
/// closed on every path out.
pub(crate) fn capture(
    program: &str,
    args: &[&str],
    input: Option<&[u8]>,
    deadline: Duration,
) -> Result<Vec<u8>> {
    let end = Instant::now() + deadline;
    let mut child = spawn(
        Command::new(program)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    )?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let pid = child.id();
    let mut tool = Tool::new(child);
    let mut stdout = stdout.ok_or("missing tool stdout")?;
    sys::set_nonblocking(stdout.as_fd())?;
    // The child is unreaped, so `pid` is still its own.
    let exited = sys::pidfd_open(pid)?;
    // The input still to write, and the pipe it goes into; dropped, which
    // closes the pipe, once it is all written or the tool stops taking it.
    let mut feed = match input {
        Some(input) => {
            let stdin = stdin.ok_or("missing tool stdin")?;
            sys::set_nonblocking(stdin.as_fd())?;
            Some((stdin, input))
        }
        None => None,
    };
    // A tool that stops reading is judged by how it exits, so a failed write
    // is reported only if the tool succeeds.
    let mut unwritten = None;
    let mut bytes = Vec::new();
    let mut eof = false;
    let outcome: Result<()> = loop {
        if let Some((stdin, rest)) = &mut feed {
            match stdin.write(rest) {
                Ok(written) => *rest = &rest[written..],
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => {
                    unwritten = Some(error);
                    feed = None;
                }
            }
            if feed.as_ref().is_some_and(|(_, rest)| rest.is_empty()) {
                feed = None;
            }
        }
        if !eof {
            // Straight into the returned buffer, never more than one byte past
            // the limit.
            let room = (LIMIT + 1 - bytes.len()) as u64;
            match (&mut stdout).take(room).read_to_end(&mut bytes) {
                Ok(_) if bytes.len() > LIMIT => break Err("clipboard exceeds 64 MiB".into()),
                Ok(_) => eof = true,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => break Err(error.into()),
            }
        }
        let reaped = match tool.try_reap() {
            Ok(Some(status)) if !status.success() => {
                break Err(format!("{program} failed; check it on Fedora").into());
            }
            Ok(Some(_)) if eof && feed.is_none() => break Ok(()),
            Ok(status) => status.is_some(),
            Err(error) => break Err(error.into()),
        };
        let remaining = end.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break Err(format!("{program} timed out").into());
        }
        // An exited child's pidfd stays readable, and so does a closed pipe,
        // so each is watched only until it has said what it has to say.
        let mut fds = [
            Poll::optional((!eof).then(|| stdout.as_fd()), Interest::Read),
            Poll::optional((!reaped).then(|| exited.as_fd()), Interest::Read),
            Poll::optional(
                feed.as_ref().map(|(stdin, _)| stdin.as_fd()),
                Interest::Write,
            ),
        ];
        match sys::poll(&mut fds, Some(remaining)) {
            Err(error) if error.kind() != io::ErrorKind::Interrupted => break Err(error.into()),
            _ => {}
        }
    };
    outcome?;
    match unwritten {
        Some(error) => Err(error.into()),
        None => Ok(bytes),
    }
}

#[cfg(test)]
mod tests;
