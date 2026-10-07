# Decision elimination review — `host-initial-refactoring-r1`

Lens: does machinery or a constraint in the current host still have a surviving justification? Files were read in the state of round `host-initial-refactoring-r1`: branch `host/initial-refactoring`, the implementation from the range under review, the register `.plan/00-index.md` with the `D-3` and `D-6` §Adjudicated notes, and `.plan/host/initial-refactoring/plan.md`.

## Scope

**Covered.**

- **Forward, inactive decisions.** The ledger holds `D-1`–`D-10`, and all ten are `Accepted`. None is superseded, rejected or withdrawn, so this half has no input. **The forward pass over inactive decisions found no surviving machinery, because there are no inactive decisions.**
- **Forward, retired fragments of active decisions.** Three dated notes retire or correct normative content:
  - `D-6` §Adjudicated retires the implied mechanism (reset a socket timeout to the remaining time before each call) and corrects the premise that a socket timeout bounds a single call. Searched `crates/`, `README.md`, `CONTRIBUTING.md` and `.agents/docs/` for `set_read_timeout`, `set_write_timeout`, `SO_SNDTIMEO` and per-call timeout wording. No socket-timeout call survives. The corrected premise survives in one code comment (`der-2`).
  - `D-3` §Adjudicated retires the old precedence, where the listing came before refusing an unsupported `type`. No code, test or mock depends on the old order. `protocol/tests.rs` does not pin it, and `bridge.py`'s `IMAGE_TYPES` is the set `image::Format` accepts, as the adjudication states. Nothing survives.
  - `D-1` §Implemented, the 2026-10-07 note, introduces no machinery.
  - The `decisions.md` dated note on the watchdog alternative introduces no machinery.
- **Backward**, from each mechanism in `crates/host/src/` to the decision or assumption it rests on:
  - `cli`: the `Command` enum and the usage line (`D-9`).
  - `image`: the separate `Jpg` variant, which is still needed because the exact offered string is requested (`D-3`).
  - `protocol`: hand-built parsing instead of derive (`I-3`), the 4096-byte limit, and vectored framing (`D-6`).
  - `clipboard`: byte-wise `lines` (`D-5`), and the 4 s and 1 s deadlines (`D-4`).
  - `process`: a process group per tool, `Tool`/`State`, the pidfd plus `poll` capture, and the ordering of the stdin-writer join (`D-4`). The pidfd goes through `libc::syscall` because `linux_pidfd` is unstable (`I-2`; the toolchain is still 1.98.1).
  - `server`:
    - the non-blocking listener, which still covers a connection that vanishes between `poll` and `accept`;
    - `Deadline` (`D-6`);
    - `Bridge` field order and `Slot` (`D-7`);
    - `Arc<UnixStream>`, which carries a refusal after a failed worker start (`D-7`);
    - scoped workers (`D-7`);
    - the main-thread watcher spawn (`D-8`).
  - `sys`:
    - the leaked wake-pipe writer;
    - `SA_RESTART` (`der-1`);
    - `poll` rounding;
    - `kill_with_parent`'s `getppid` check (`D-8`);
    - `peer_uid` kept on `libc` (`I-2`).
  - Manifests: the workspace `[lints]`, `rust-version = 1.89` (`File::try_lock`; `io::pipe` needs only 1.87), and `clippy.toml`'s `allow-unwrap-in-tests`, which the sibling `tests.rs` modules use.
  - `README.md`: the Linux 5.3 requirement, which `pidfd_open` still needs.
  - `CONTRIBUTING.md` §Tests: both `D-2` rules are current.

  Apart from `der-1` and `der-2`, every justification checked is still alive.

**Not covered.**

- `Q-1`, by instruction.
- `bridge.py` and `test_bridge.py`, beyond the one fact the `D-3` adjudication relies on.
- The tests under `crates/host/tests/binary/` as instruments. They were read only for machinery that depends on retired content, and there is none.
- `.agents/docs/test-architecture.md` beyond searching it for retired mechanisms.
- The effort-guard plugin and `.claude/`.
- Whether a decision is implemented correctly. That is another lens.

## Findings

### der-1 — `SA_RESTART` and the `accept` loop's `Interrupted` retry rest on a premise that never held: a non-blocking `accept` cannot fail with `EINTR`

**Tier C.**

**Finding.** `sys::wake_on_termination` installs the handlers with `SA_RESTART`. Its doc comment says this is "so interrupted reads and writes resume". `server::serve` also lists `io::ErrorKind::Interrupted` among the `accept` errors it retries. Both trace to one premise, stated in `analysis.md` §Server lifecycle and carried into `F-6`: "Handlers are installed without `SA_RESTART`, so a signal that lands between a ready `poll` and `accept` makes `accept` fail with `EINTR`". `D-7`'s required property follows from it: "An `accept` error from a connection that vanished, or `EINTR`, is retried". The listener was non-blocking both before and after the refactoring (original `main.rs:339` and current `server::serve`). A non-blocking `accept` never sleeps, so it cannot be interrupted, and the premise did not hold even for the original code.

**Current behavior / contract.**

- The accept loop polls with no timeout and retries `Interrupted` from `poll`, which is necessary. It then calls `accept` on a non-blocking listener.
- `SA_RESTART` affects only syscalls that sleep and are restartable. In the serving process every socket and tool-stdout descriptor is non-blocking. Each remaining blocking call goes through a `std` path that already retries `EINTR`: the stdin writer's `write_all`, `Child::wait` in `Tool::drop`, and `Command::spawn`. `poll` is never restarted whatever the flag says.
- `SA_RESTART` and the `Interrupted` arm on `accept` therefore have no observable effect, and the doc comment names a purpose that nothing in the process has.

**Why it is a problem.** Each mechanism carries a stated reason that is false. A future reader reasons from those reasons. For example, a reader may conclude that dropping `SA_RESTART` would reintroduce `F-6`'s exit-status bug, or that a blocking read somewhere in `serve` depends on it. `F-6`'s third bullet and `D-7`'s property also still describe the `EINTR`-on-`accept` failure as real. This is internal only: nothing a user observes changes, and no instrument depends on it.

**Evidence / reproduction.** A C probe was run in scratch and not retained. It sets up an `AF_UNIX` listener, a `SIGTERM` handler installed **without** `SA_RESTART`, a thread that sends `pthread_kill(main, SIGTERM)` in a tight loop, and, in the non-blocking case, a thread that connects and closes in a loop. Run on the repository's devcontainer, kernel 7.2.7:

| Listener     | `accept` calls | Succeeded | `EAGAIN` | `EINTR` | Handler runs |
| ------------ | -------------- | --------- | -------- | ------- | ------------ |
| non-blocking | 200,000        | 177,595   | 22,405   | **0**   | 7,376,904    |
| blocking     | 2,000          | 0         | 0        | 2,000   | 22,248       |

The blocking control shows that the handler interrupts a sleeping `accept` every time. The non-blocking listener, which is what `serve` uses, never returned `EINTR` across 7.4 million handler runs. The causal chain:

- the initial implementation's `crates/host/src/main.rs`, in the base state of the landing round `host-initial-refactoring-r1` reviewed, line 339: `listener.set_nonblocking(true)?`;
- `analysis.md` line 103 states the premise;
- `F-6` carries it;
- `D-7` §Implemented: "installs SIGTERM, SIGINT and SIGHUP handlers with `SA_RESTART`";
- `sys.rs` lines 35–36 and 49;
- `server.rs` lines 241–249;
- the journal (line 97) records that this path "cannot be provoked deterministically from a test".

**Required property.** Every mechanism in the signal and accept path, and the comment that justifies it, describes a failure the serving process can actually meet. Whether the `D-7` property's `EINTR` clause and `F-6`'s third bullet should keep describing that failure is the architect's decision. This review does not amend them.

### der-2 — `server::Deadline`'s doc comment restates the `D-6` premise that the adjudication falsified

**Tier C.**

**Finding.** The doc comment on `server::Deadline` (`server.rs` lines 37–40) opens: "A socket timeout bounds a single call, and the kernel re-arms it within one large write". `D-6` §Adjudicated corrects exactly that premise: "A socket timeout does not bound even one call". The second clause of the comment is the correction, so the comment asserts both the retired premise and its refutation.

**Current behavior / contract.** `Deadline` polls a non-blocking stream for the time that remains, which is the adjudicated mechanism. No socket-timeout code survives. Only the justification text carries the retired fragment.

**Why it is a problem.** The comment is the justification a maintainer meets at the mechanism, and its first clause is the claim the record withdrew. It invites someone to "simplify" back to per-call timeouts for small writes, which is the approach the implementer's first attempt took and that failed (journal line 95). This is internal only.

**Evidence / reproduction.** `D-6` §Adjudicated (2026-10-07) records a probe: one blocking 8 MiB `send` with `SO_SNDTIMEO` at 200 ms took 6.30 s against a slow reader. `decisions.md` §Connection deadlines and framing rewrote its own wording that rested on the same premise, with a dated note. The code comment was not rewritten in the same way.

**Required property.** The stated reason for `Deadline` matches the adjudicated premise: a socket timeout does not bound one call.

## Count

Two findings: Tier A 0, Tier B 0, Tier C 2. There are no inactive decisions. In the retired fragments of active decisions, only the `D-6` premise survives in the current system (`der-2`). `der-1` is a premise that never held, found in the backward pass.
