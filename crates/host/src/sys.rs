//! Every call into `libc`, each behind a safe function. This is the only module
//! the crate's lints permit `unsafe` in.
#![allow(unsafe_code)]

use std::{
    io::{self, PipeReader},
    marker::PhantomData,
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
        unix::{net::UnixStream, process::CommandExt},
    },
    process::Command,
    sync::atomic::{AtomicI32, Ordering},
    time::Duration,
};

/// The write end of the wake pipe, or -1 before [`wake_on_termination`].
static WAKE: AtomicI32 = AtomicI32::new(-1);

extern "C" fn wake(_: libc::c_int) {
    let fd = WAKE.load(Ordering::Relaxed);
    // SAFETY: `write` and the errno accessor are async-signal-safe, and the
    // byte outlives the call. errno is restored so that the interrupted thread
    // sees its own.
    unsafe {
        let errno = libc::__errno_location();
        let saved = *errno;
        libc::write(fd, [1_u8].as_ptr().cast(), 1);
        *errno = saved;
    }
}

/// Makes SIGTERM, SIGINT and SIGHUP write a byte to a pipe and returns its
/// read end, which is readable from then on, whichever thread the signal
/// reached. A handler that runs during a [`poll`] makes it fail with
/// `Interrupted`, and its callers retry; every other call that can block in
/// the serving process goes through `std`, which retries `EINTR` itself.
pub(crate) fn wake_on_termination() -> io::Result<PipeReader> {
    let (reader, writer) = io::pipe()?;
    // A full pipe already holds a wake-up, so the handler never blocks.
    set_nonblocking(writer.as_fd())?;
    // Never closed: a handler may run at any point until the process exits,
    // and must not write to a descriptor that has been reused.
    WAKE.store(writer.into_raw_fd(), Ordering::Relaxed);
    // SAFETY: `action` is a valid, zero-initialised `sigaction` whose handler
    // does only async-signal-safe work, and the old-action pointers are null.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = wake as *const () as usize;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            if libc::sigaction(signal, &action, std::ptr::null_mut()) < 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(reader)
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

    /// Whether the last [`poll`] found this descriptor ready.
    pub(crate) fn ready(&self) -> bool {
        self.0.revents != 0
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

/// Makes the child `command` starts receive SIGKILL when this process ends,
/// however it ends. The kernel sends the signal when the spawning *thread*
/// exits, so `command` must be spawned from a thread that lives as long as the
/// child is wanted. A parent that dies between `fork` and the `prctl` would be
/// missed, so the child checks afterwards and fails its spawn if it has
/// already been orphaned.
pub(crate) fn kill_with_parent(command: &mut Command) -> &mut Command {
    let parent = std::process::id();
    // SAFETY: the closure runs in the child between `fork` and `exec`, and
    // calls only `prctl` and `getppid`, which are async-signal-safe, and
    // builds `io::Error`s that do not allocate.
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() as u32 != parent {
                return Err(io::ErrorKind::NotFound.into());
            }
            Ok(())
        })
    }
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
