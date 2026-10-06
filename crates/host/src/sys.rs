//! Every call into `libc`, each behind a safe function. This is the only module
//! the crate's lints permit `unsafe` in.
#![allow(unsafe_code)]

use std::{
    io,
    marker::PhantomData,
    os::{
        fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
        unix::net::UnixStream,
    },
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn stop(_: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

/// Makes SIGTERM and SIGINT clear the flag [`running`] reads.
pub(crate) fn stop_on_termination() -> io::Result<()> {
    // SAFETY: `action` is a valid, zero-initialised `sigaction` whose handler
    // only stores to an atomic, which is async-signal-safe.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = stop as *const () as usize;
        libc::sigemptyset(&mut action.sa_mask);
        if libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut()) < 0
            || libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) < 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

pub(crate) fn running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

/// Waits until `fd` is readable or `timeout` passes; `Ok(false)` on timeout.
pub(crate) fn poll_readable(fd: BorrowedFd<'_>, timeout: Duration) -> io::Result<bool> {
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let millis = timeout.as_millis().min(libc::c_int::MAX as u128) as libc::c_int;
    // SAFETY: `poll` points at one initialised `pollfd`, and the count is 1.
    let ready = unsafe { libc::poll(&mut poll, 1, millis) };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ready > 0)
}

/// One descriptor, borrowed for as long as this value lives, and the readiness
/// [`poll`] waits for on it.
#[repr(transparent)]
pub(crate) struct Poll<'fd>(libc::pollfd, PhantomData<BorrowedFd<'fd>>);

/// What [`Poll`] waits for.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Interest {
    /// Readable without blocking, or hung up.
    Read,
    /// Writable without blocking, or failed.
    Write,
}

impl<'fd> Poll<'fd> {
    pub(crate) fn new(fd: BorrowedFd<'fd>, interest: Interest) -> Self {
        let events = match interest {
            Interest::Read => libc::POLLIN,
            Interest::Write => libc::POLLOUT,
        };
        Self(
            libc::pollfd {
                fd: fd.as_raw_fd(),
                events,
                revents: 0,
            },
            PhantomData,
        )
    }
}

/// Waits until one of `fds` is ready or `timeout` passes, rounding the timeout
/// up to a whole millisecond so that a short remainder never spins. `None`
/// waits without limit. Returns early with `Interrupted` when a signal lands.
pub(crate) fn poll(fds: &mut [Poll<'_>], timeout: Option<Duration>) -> io::Result<()> {
    let millis = match timeout {
        None => -1,
        Some(timeout) => timeout
            .as_nanos()
            .div_ceil(1_000_000)
            .min(libc::c_int::MAX as u128) as libc::c_int,
    };
    // SAFETY: `Poll` is `repr(transparent)` over `pollfd`, so the slice is
    // `fds.len()` contiguous, initialised `pollfd` values, and each descriptor
    // stays open because `Poll` borrows it.
    let ready = unsafe {
        libc::poll(
            fds.as_mut_ptr().cast::<libc::pollfd>(),
            fds.len() as libc::nfds_t,
            millis,
        )
    };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn set_nonblocking(fd: BorrowedFd<'_>) -> io::Result<()> {
    let fd = fd.as_raw_fd();
    // SAFETY: `fd` is borrowed, so it is open for the duration of both calls;
    // F_GETFL and F_SETFL take no pointer arguments.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    // SAFETY: as above.
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// A descriptor that becomes readable when process `pid` exits. It refers to
/// that process and no other, even after its PID is reused.
pub(crate) fn pidfd_open(pid: u32) -> io::Result<OwnedFd> {
    // SAFETY: `pidfd_open` takes a PID and flags, and no pointers.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel has just returned this descriptor, open and
    // close-on-exec, and nothing else owns it.
    Ok(unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
}

/// Sends SIGKILL to the process group `group`. The caller guarantees the group
/// is still its own, which holds while its leader is unreaped.
pub(crate) fn kill_group(group: u32) {
    // SAFETY: `kill` takes no pointers; a stale group is the caller's contract.
    unsafe {
        libc::kill(-(group as libc::pid_t), libc::SIGKILL);
    }
}

/// The real UID of this process.
pub(crate) fn uid() -> u32 {
    // SAFETY: `getuid` takes no arguments and cannot fail.
    unsafe { libc::getuid() }
}

/// The UID of the process at the other end of `stream`, as the kernel recorded
/// it when the connection was made.
pub(crate) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `credentials` and `size` are live locals, and `size` is the size
    // of the buffer `credentials` provides, as SO_PEERCRED requires.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(credentials.uid)
}

#[cfg(test)]
mod tests;
