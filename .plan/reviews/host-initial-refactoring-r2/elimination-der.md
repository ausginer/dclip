# Decision elimination review — `host-initial-refactoring-r2`

Lens: does machinery or a constraint in the current system still have a surviving justification, now that `D-11` supersedes `D-7`, `D-12` supersedes `D-9`, and round `host-initial-refactoring-r1`'s findings `F-10`–`F-24` have been ruled and remediated?

Files were read in the state round `host-initial-refactoring-r2` reviews: branch `host/initial-refactoring` after `plan.md` §Phase 5, with the register `.plan/00-index.md`, `plan.md`, `journal.md`, `decisions.md` and `analysis.md` under `.plan/host/initial-refactoring/`, and round `host-initial-refactoring-r1`'s published reports. Probes ran against a debug build of that state, made in a scratch target directory. One build was a scratch mutant, described in `der-2`. None of the probes was retained.

## Scope

**Covered.**

- **Forward, inactive decisions.** The ledger has two: `D-7`, superseded by `D-11`, and `D-9`, superseded by `D-12`.
  - **`D-7`.** Its retired fragments are listed below, each with where it stands now:
    - **Unconditional handling of SIGTERM, SIGINT and SIGHUP.** Gone from `sys::wake_on_termination`, but it survives as the expectation of a binary test (`der-1`).
    - **`SA_RESTART`, and the `EINTR` retry on `accept`.** Gone from code and comments.
    - **Joining the workers before the socket is dropped.** Gone. `Bridge` now drops inside the scope.
    - **"Setting its timeouts" as a per-connection failure.** Gone. No `set_read_timeout` or `set_write_timeout` remains, and the arm is now `set_nonblocking`.
    - **"An `accept` error from a connection that vanished … is retried".** `D-11` restates it, and the code carries it with its reason (`der-2`).

    The fragments `D-11` keeps (the wake pipe, the scope, the cleanup order and the limit of 16) were traced backward and are justified.
  - **`D-9`.** `handle-stdio` stays deleted, and `D-12` keeps that. The rule that every other invocation prints the usage line was retired, and `cli::parse` now names the argument at fault. The separate `handle-stdio` test is gone, and that word is now a row in `should_print_usage_and_exit_1_for_an_unpublished_invocation`. **For `D-9`, the forward pass found no surviving machinery.**
- **Forward, retired fragments of active decisions:**
  - **`D-4` §Adjudicated and §Remediated** retired the stdin writer thread, and with it the constraint "joined after the child is killed, never before". Nothing in the code survives. The constraint survives as a model comment in `.agents/docs/documentation.md` §5.1 (`der-4`).
  - **`D-6` §Adjudicated** withdrew the premise that a socket timeout bounds a single call. The doc comment on `server::Deadline` now states the corrected premise, and no socket-timeout call remains. Nothing survives.
  - **`D-6`'s corrected header spelling (`F-16`).** `protocol::write_response` relies on `serde_json` without `preserve_order`, and the binary tests assert the sorted form. Nothing survives.
  - **`D-8` §Adjudicated** narrowed the last property to the death of the serving process. The doc comment on `clipboard::watch` states both endings. No test asserts that an in-flight sync survives a graceful shutdown. Nothing survives.
  - **`F-6`'s withdrawn `EINTR` clause**, and its counterpart in `analysis.md`. No code rests on it. The remaining limb of the note, "a fatal non-`WouldBlock` `accept` error such as `ECONNABORTED`", is the premise in `der-2`.
  - **`F-14` §Ruling's waived arms.** No test or machinery was added for them, and none was owed.
  - **`D-3` §Adjudicated** and **`D-13`.** Neither introduces machinery in the system under review.
- **Backward, from Phase 5's machinery to what it rests on:**
  - **`sys`:**
    - the disposition read in `wake_on_termination` (`D-11`);
    - errno being saved in the handler, which may run on any thread;
    - the leaked wake-pipe writer;
    - `Poll::optional`, which lets one array watch a changing subset of its descriptors;
    - the rounding up in `poll`.
  - **`server`:**
    - `Bridge` holding only the watcher and the `Socket`, and the explicit `drop(lock)` after the scope (`D-11`);
    - `Deadline::or_until_shutdown`, and the wake pipe never being drained (`D-11`, plan note 5.4);
    - the `Arc<UnixStream>` that carries a refusal after a failed worker start (`D-11`'s per-connection arm);
    - the refusal site's comment about the blocking assumption (`F-20`);
    - the `accept` arms (`der-2`).
  - **`process`:**
    - the non-blocking stdin written in the `poll` loop (`D-4` §Adjudicated);
    - `unwritten`, the deferred report of a failed write. A tool that stops reading is still judged by its exit status;
    - the `Interrupted` retry. `capture` runs inside the serving process, where handled signals land.
  - **`protocol::write_response`.** Its `Interrupted` arm is justified by the generic `Write` contract it is written against, not by the socket path. It does not rest on the premise `F-21` withdrew.
  - **`cli`.** `not a UID: <value>` (`D-12`).
  - **`main`.** The `Result` alias without `Send + Sync` (`F-23`).
  - **Binary support:**
    - `Scratch::run` and `Scratch::serve` under `PATIENCE`, and the `Running` guard (`F-15`, `D-2`);
    - `serve_ignoring` (`D-11`'s witness).
- **The two assumptions in the journal's Phase 5 entry:**
  - the concurrency test's retry for a refusal lost to a reset (`der-3`);
  - the termination-signal tests' assumption about inherited dispositions (`der-1`).

**Not covered.**

- `Q-1`, which is out of scope by instruction.
- `bridge.py`, except to establish what order it sends and reads in (`der-3`).
- The effort guard under `.claude/`.
- `setup-host.sh` and the Dockerfile, beyond the searches for retired premises.
- The decisions' alternatives in `decisions.md`, except where a retired fragment pointed into them.
- Whether Phases 1–4's decisions are themselves still justified, apart from the fragments `D-11` and `D-12` carried forward. Round `host-initial-refactoring-r1` covered that.
- Any rerun of `I-1`, `I-2` or `I-3`.

## Findings

| Local id | Tier | Claim |
| -------- | ---- | ----- |
| `der-1`  | B    | `should_remove_the_socket_and_exit_0_on_a_termination_signal` still expects `D-7`'s unconditional handling, so it fails a correct bridge whenever the test runner inherits SIGHUP or SIGINT as ignored |
| `der-2`  | C    | The `accept` loop gives its `WouldBlock` retry a reason that `AF_UNIX` does not have, and keeps a `ConnectionAborted` retry with no evidence that an `AF_UNIX` `accept` can return it. What the `WouldBlock` arm really carries, a signal that interrupts `poll`, is stated nowhere |
| `der-3`  | C    | The concurrency test's retry for a failed exchange, and the journal's claim that a refusal can be lost to a reset, rest on a premise the kernel does not have: the refusal is always read, and only end of stream becomes `ECONNRESET` |
| `der-4`  | C    | `documentation.md` §5.1's model comment states the stdin-writer join constraint, which `D-4` §Remediated removed from the code |

---

### der-1 — The termination-signal test asserts the unconditional handling that `D-11` retired

**Tier B.** An instrument the repository relies on fails a correct bridge.

**Finding.** `crates/host/tests/binary/server.rs` `should_remove_the_socket_and_exit_0_on_a_termination_signal` starts `serve` through `Scratch::serve`, once for each of TERM, INT and HUP. It asserts that each signal ends `serve` with status 0. The child inherits the test process's signal dispositions, because `Command` resets SIGPIPE only. The test is therefore correct only if the runner did not inherit SIGINT or SIGHUP as ignored.

**Current behavior / contract.** `D-11`: _"The handled signals are SIGTERM, SIGINT and SIGHUP, each only if its disposition is not ignored when `serve` starts. A signal inherited as ignored stays ignored, and the bridge does not stop for it."_ The test's expectation is `D-7`'s retired form: _"SIGTERM, SIGINT or SIGHUP leads to shutdown as soon as the signal is delivered"_, without condition. `D-7` §Implemented names this test as `D-7`'s witness "(now with SIGHUP)", and `D-11` §Implemented lists it again without restating what it now assumes.

**Why it is a problem.** The test's justification was `D-7`'s unconditional property, and that property is gone. The behaviour the test now pins is conditional on the environment, and the test neither establishes nor states the condition.

- **Two ordinary launches inherit an ignored disposition.** A run under `nohup` ignores SIGHUP. A run as a background job of a non-interactive shell ignores SIGINT, so `cargo test &` in a script does.
- **In both, a correct bridge fails the test**, with `serve did not exit` after `PATIENCE`.
- **The journal records the effect** under "Noticed, not acted on". Neither the test nor the register states it as a precondition.

`D-2` requires the layer to run "in the default `cargo test` on a fresh checkout" with nothing provisioned. Default signal dispositions are now an unstated provision.

**Evidence / reproduction.** Run in round `host-initial-refactoring-r2`'s reviewed state, on the repository's devcontainer (kernel 7.2.7), with `CARGO_TARGET_DIR` in scratch:

- `cargo test --test binary should_remove_the_socket_and_exit_0`: passed, in 0.04 s.
- `sh -c "trap '' HUP; exec cargo test --test binary should_remove_the_socket_and_exit_0"`: **failed** after 10.03 s.
- `sh -c "cargo test --test binary should_remove_the_socket_and_exit_0 & wait"`, where the background job inherits SIGINT as ignored: **failed** after 10.02 s, panicking at `support.rs` in `Server::wait` (`serve did not exit`).
- Control: `should_keep_a_signal_ignored_when_serve_inherits_it_ignored` passes under `trap '' HUP`, in 0.22 s. The bridge behaves as `D-11` requires, and only the test's expectation is stale.

**Required property.** The binary tests that pin `D-11`'s signal handling assert what `D-11` requires, whatever dispositions the test runner inherited. Where a test depends on the runner's dispositions, that dependency is stated where the test can be found.

---

### der-2 — The `accept` loop's retries rest on a TCP premise, and the arm that matters carries the wrong reason

**Tier C.** Internal only: no user-observable effect today.

**Finding.** In `crates/host/src/server.rs` `serve`, the accept loop retries `io::ErrorKind::WouldBlock` and `io::ErrorKind::ConnectionAborted`, with the comment _"The readiness was taken by a connection that vanished. The listener is non-blocking, so `accept` never sleeps and a signal cannot interrupt it."_ `D-11` makes this a required property: _"`accept`: `WouldBlock` is retried (the readiness was taken by a connection that vanished), and so is `ConnectionAborted`."_

- **The clause descends from `D-7`.** `D-7` had "An `accept` error from a connection that vanished … is retried". `F-6`'s 2026-10-08 note keeps "a fatal non-`WouldBlock` `accept` error such as `ECONNABORTED`" as a live limb of the initial implementation's defect.
- **The premise is TCP's.** There, a connection reset before it is accepted can leave a stale readiness or fail `accept` with `ECONNABORTED`. On Linux `AF_UNIX`, a peer that connects and goes away stays queued, and `accept` returns it.

**Current behavior / contract.**

- **The `WouldBlock` arm is load-bearing, but for another reason.** A handled signal that lands on the main thread fails `poll` with `Interrupted`, and the loop falls through: the arm matches anything other than a non-`Interrupted` error. `ready[1]` reports nothing, because `poll` wrote no `revents`, and `accept` is then called with nothing queued, which gives `EAGAIN`. Only the `WouldBlock` arm returns the loop to `poll`, where the wake pipe is seen.
- **The `ConnectionAborted` arm** answers an error that this socket family has not been shown to produce.

**Why it is a problem.** Both arms carry a justification that does not hold for the socket the bridge uses. The one real dependency, a clean exit on a termination signal, is stated nowhere: not in the comment, not in `D-11`, and not in the doc comment on `sys::wake_on_termination`, which says only that `poll`'s callers retry. A reader who checks the stated reason will find it false for `AF_UNIX`, and may remove the `WouldBlock` arm as dead. That turns every termination signal that reaches the main thread of an idle bridge into exit status 1. This is the class of `F-21`, where a retry was justified by a failure the process cannot meet. Here, one arm's real purpose is also hidden behind the false reason. Round `host-initial-refactoring-r1`'s backward pass accepted the listener's handling of "a connection that vanishes between `poll` and `accept`" without testing that premise.

**Evidence / reproduction.** Run in round `host-initial-refactoring-r2`'s reviewed state, on the repository's devcontainer, kernel 7.2.7:

- **A vanished peer is accepted, not dropped.** A Python probe used a non-blocking `AF_UNIX` listener, with backlog 16. A client connected and then (a) closed, (b) sent a line and closed, or (c) shut down both directions and closed. After `select` reported the listener readable, `accept` returned the connection in 200 of 200 trials of each shape: 600 of 600 in all. There was no `EAGAIN` and no `ECONNABORTED`; in (b) the line was still readable.
- **The `WouldBlock` arm is what makes a signal exit clean.** In a scratch copy of the reviewed state, `io::ErrorKind::WouldBlock` was removed from the arm, and `ConnectionAborted` kept. `serve` was started idle and sent SIGTERM 20 times:
  - Reviewed state: exit 0 with empty stderr, 20 of 20.
  - Mutant: exit 1, 20 of 20, with `claude-clipboard-host: Resource temporarily unavailable (os error 11)`.
- **`ConnectionAborted`:** the only evidence offered is the probe above, which found none. From reading the upstream `unix_accept` source, which was not verified in this environment, its error paths are `EOPNOTSUPP`, `EINVAL` and the `skb_recv_datagram` result (`EAGAIN`, or `EINTR` when blocking). It has no `ECONNABORTED` path. The record cites no probe or source for that path.

**Required property.** Each retry arm in the `accept` loop, and the record's statement of it in `D-11`, names a failure that a non-blocking `AF_UNIX` listener in this process can actually meet. The dependency of a clean signal exit on the fall-through from an interrupted `poll` is stated where that path is enforced.

---

### der-3 — The concurrency test tolerates a lost refusal that the kernel never produces

**Tier C.** No runtime behaviour changes, and the instrument still detects what it is meant to detect. What is wrong is a stated premise, and a claimed user-visible defect that the evidence refutes.

**Finding.** In `crates/host/tests/binary/server.rs`, the retry loop of `should_refuse_a_connection_over_the_limit_and_keep_serving` treats any failed exchange as one more attempt. Its comment reads: _"A refused connection is closed with the request unread, which can reset it before the refusal is read, so a failed exchange is one more attempt."_

The journal's Phase 5 entry, under "Noticed, not acted on", generalises this into a runtime claim:

> "If the peer has already sent the request, Linux resets the connection, and the peer's read can fail with `ConnectionReset` before the refusal is read … a refused UID, whose message carries the `--allow-uid` remedy, may reach the container as a reset instead."

**Current behavior / contract.** On Linux `AF_UNIX`, closing a socket with unread data in its receive queue sets `ECONNRESET` on the peer. The peer still reads every byte already queued to it first. Only the read that would have returned end of stream returns `ECONNRESET` instead. The refusal is written before the close, so it is always ahead of the reset.

- **The test reads to end of stream.** It uses `read_to_end` in the loop, and `support::read_response` does the same. The read after the refusal fails, and the test discards an exchange whose refusal is already in `bytes`, because `read_to_end` appends what it read before the error.
- **The shim reads by the framing instead.** `bridge.py` reads one header line and then `size` bytes, so it never makes the read that fails.

**Why it is a problem.** The machinery and the claim rest on an ordering the kernel does not have, so their justification is gone.

- **The tolerance is wider than any real failure.** It accepts a failed exchange as normal, where the only real failure is end of stream replaced by a reset. Its stated reason would also lead a reader to believe the server's refusals are unreliable.
- **The journal's claim would misdirect the next round.** A reader acting on "a refused UID may reach the container as a reset" could schedule server work, such as draining the request before refusing, for a container-visible defect that the shim does not exhibit.

**Evidence / reproduction.** Run in round `host-initial-refactoring-r2`'s reviewed state, on the repository's devcontainer, kernel 7.2.7:

- **Socket pair, client sends first, then the server writes a refusal line and closes without reading:**
  - Read by a `recv` loop to end of stream: `ConnectionResetError` 200 of 200. The refusal was returned by the first `recv`, and the exception came on the next.
  - Read with `makefile("rb").readline()`, as `bridge.py` does: the refusal line, 100 of 100 without a socket timeout and 100 of 100 with `settimeout(12)`.
- **Control, server reads the request before closing:** data and a clean end of stream, 200 of 200.
- **The real binary, `serve` with 16 idle connections holding every place:**
  - The shim, `bridge.py` invoked as `wl-paste --list-types`: `wl-paste: too many concurrent clipboard requests`, 30 of 30.
  - A raw client that sends and then reads to end of stream: `ConnectionResetError`, 50 of 50, with the same outcome when the read is delayed 50 ms.
  - A client that sends after the close: `BrokenPipeError`, 50 of 50.

**Required property.** The concurrency test's tolerance and its stated reason describe a failure that a peer can actually meet. The record does not carry, as a user-visible defect, a lost refusal that the shim's read order cannot exhibit.

---

### der-4 — `documentation.md`'s model comment states a constraint that `D-4` retired

**Tier C.** Internal only.

**Finding.** `.agents/docs/documentation.md` §5.1, under "State a constraint that holds and the consequence of breaking it", gives as its working example:

> _The child is killed before the stdin writer is joined; joining first blocks forever on a tool that stopped reading_

The example came from the initial implementation, whose `capture` had a comment "Kill before joining a possibly blocked stdin writer on failure." `D-4` carried it as a required property. `D-4` §Adjudicated then showed it was insufficient: a reaped tool is not killed, and the join still blocked (`F-13`). §Remediated removed the writer thread, so the constraint now holds "by construction" because nothing is joined.

**Current behavior / contract.** `process::capture` has no writer thread and no join. Its doc comment states the constraint that replaced the old one: stdin is written from the `poll` loop, and the pipe closes on every path out.

**Why it is a problem.** The normative document's model of "a constraint that holds" is a constraint that no longer exists in the tree. It is also one that `D-4` §Adjudicated found did not guard what it claimed to guard. A reader taking the example as a pattern, or as a description of `capture`, learns a retired design. The neighbouring example of a bare pointer, `(D-7)`, likewise points at a superseded decision. As an illustration of form only, that pointer misleads less.

**Evidence / reproduction.**

- `.agents/docs/documentation.md` line 100 holds the example. A text search finds it unchanged since the initial implementation.
- A search of `crates/host/src/process.rs` for `writer` or `join` finds no writer thread and no join in the reviewed state.
- `D-4` §Remediated reads: "there is no writer thread and nothing to join".

**Required property.** The examples that `documentation.md` gives of a working comment state constraints that hold, or are plainly illustrations that do not depend on the tree.

---

## Null results stated

- **`D-9`:** the forward pass found no surviving machinery.
- **`D-7`:** the forward pass found surviving machinery in `der-1` and `der-2` only. `SA_RESTART`, the `Interrupted` arm on `accept`, join-before-drop and the per-connection socket timeouts are all gone.
- **Retired fragments of active decisions:** the forward pass found surviving content only in `der-4`. Nothing survives from `D-6`'s withdrawn premise, `D-6`'s header correction, `D-8`'s narrowed property, or `F-6`'s withdrawn `EINTR` clause.
- **Backward pass over Phase 5's machinery:** apart from `der-2` and `der-3`, every mechanism listed under Scope has a live justification.
