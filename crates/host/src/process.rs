//! Running an external tool under a deadline, and making sure it is reaped.

use crate::{Result, sys};
use std::{
    io::{self, Read, Write},
    os::{fd::AsFd, unix::process::CommandExt},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// The most a tool may print, which is also the largest image the bridge serves.
pub(crate) const LIMIT: usize = 64 * 1024 * 1024;

/// A spawned tool, leader of its own process group. Dropped while unreaped,
/// it kills the group and reaps the child, so no path out of a capture leaves
/// a tool behind.
pub(crate) enum Tool {
    Unreaped(Child),
    Reaped,
}

impl Tool {
    pub(crate) fn spawn(command: &mut Command) -> io::Result<Self> {
        command.process_group(0).spawn().map(Self::Unreaped)
    }
}

impl Drop for Tool {
    fn drop(&mut self) {
        if let Self::Unreaped(child) = self {
            sys::kill_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Runs `program`, feeds it `input`, and returns what it prints, failing if it
/// exits unsuccessfully, prints more than [`LIMIT`] or outlasts `timeout`.
pub(crate) fn capture(
    program: &str,
    args: &[&str],
    input: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut tool = Tool::spawn(
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
    let (stdin, stdout) = match &mut tool {
        Tool::Unreaped(child) => (child.stdin.take(), child.stdout.take()),
        Tool::Reaped => (None, None),
    };
    let mut stdout = stdout.ok_or("missing tool stdout")?;
    sys::set_nonblocking(stdout.as_fd())?;
    let writer = match input {
        Some(bytes) => {
            let mut stdin = stdin.ok_or("missing tool stdin")?;
            Some(thread::spawn(move || stdin.write_all(&bytes)))
        }
        None => None,
    };
    let started = Instant::now();
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 16 * 1024];
    let mut eof = false;
    let outcome: Result<()> = (|| {
        loop {
            if started.elapsed() >= timeout {
                return Err(format!("{program} timed out").into());
            }
            if !eof {
                match stdout.read(&mut chunk) {
                    Ok(0) => eof = true,
                    Ok(n) => {
                        if bytes.len() + n > LIMIT {
                            return Err("clipboard exceeds 64 MiB".into());
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            if let Tool::Unreaped(child) = &mut tool
                && let Some(status) = child.try_wait()?
            {
                if !status.success() {
                    return Err(format!("{program} failed; check it on Fedora").into());
                }
                if eof {
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
    })();
    // Kill before joining a possibly blocked stdin writer on failure.
    if outcome.is_ok() {
        tool = Tool::Reaped;
    }
    drop(tool);
    if let Some(writer) = writer {
        let written = writer.join().map_err(|_| "clipboard writer panicked")?;
        if outcome.is_ok() {
            written?;
        }
    }
    outcome?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
