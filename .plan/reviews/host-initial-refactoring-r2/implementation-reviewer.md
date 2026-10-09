# Implementation review: feature proof

Round `host-initial-refactoring-r2`, reviewer pass (feature proof). The pass read the state this round reviews: branch `host/initial-refactoring` after phase 5 of [`plan.md`](../../host/initial-refactoring/plan.md), "Remediate round `host-initial-refactoring-r1`". It checked that state against the register: `D-11`, which supersedes `D-7`; `D-12`, which supersedes `D-9`; `D-4` §Adjudicated and §Remediated; `D-8` §Adjudicated and §Remediated; `F-14` §Ruling; and the status lines of `F-10`–`F-24`. "The pre-phase-5 source" below means `crates/host/src/` as the change that mints `D-11` and `D-12` left it. It is the state phase 5 started from. The pass read no other pass's artifact or staging area.

## Scope

**Covered.**

- **Production source changed by phase 5**, read in full: `server.rs`, `sys.rs`, `process.rs`, `cli.rs`, `main.rs` and the `clipboard::watch` doc comment. `protocol::read_request` and `protocol::write_response` were read where the shutdown path and the refusal path go through them.
- **Tests.** `crates/host/tests/binary/support.rs`, `server.rs` and `cli.rs`, plus `server/tests.rs` and `process/tests.rs`. Each was checked against plan steps 5.1–5.9 and against the tests that `D-11` §Implemented, `D-12` §Implemented and `D-4` §Remediated name. Every test the register names exists.
- **Record and documentation.** The phase-5 entry in the journal, the register's §Implemented, §Remediated and status lines for this phase, `README.md` step 2 and §Tests and limitations, `test-architecture.md` §The layers with its change record, and the absence of record identifiers from `README.md` and source.
- **Gates, re-run by this pass:**
  - `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` were clean.
  - `cargo test --workspace` passed 37 unit and 19 binary tests.
  - `cargo test --locked --target x86_64-unknown-linux-musl` passed 37 and 19.
  - The Python suite reported OK.
  - The release musl binary, built from a clean export of the reviewed state, is 611,072 bytes, as the journal records.
  - `cargo tree -e normal` shows only `libc` and `serde_json` as direct dependencies.
  - `unsafe` appears nowhere outside `sys.rs`.
  - The branch matches its remote.
- **Did each failing test fail first?** HEAD's binary tests were replayed against the pre-phase-5 source in a scratchpad copy. Three tests failed there:
  - `should_keep_a_signal_ignored_when_serve_inherits_it_ignored`;
  - `should_unadvertise_at_once_and_turn_away_an_idle_peer_on_a_signal`;
  - `should_exit_1_naming_the_argument_at_fault_for_malformed_serve_arguments`.

  The other 16 passed, the concurrency test among them, as step 5.5 expects. Run against the pre-phase-5 `capture` in its old signature, `F-13`'s test shape took 3.002 s.
- **Mutation, step 5.5.** In a scratchpad copy, `admit` was replaced by an unconditional slot. `should_refuse_a_connection_over_the_limit_and_keep_serving` then failed: after 12.02 s, the seventeenth connection got `timed out` instead of the refusal.
- **The three journal items the round names:** the refusal lost to a reset (`reviewer-1`), the concurrency test's retry (`reviewer-3`, and below), and the assumption about inherited signals in the termination tests (`reviewer-2`). Each was probed against the reviewed state's debug binary, in a scratchpad directory.

**Not covered.**

- **`Q-1`.** Out of scope, as instructed.
- **The Docker image build.** Not run, because no Docker CLI is available. Its test command was run, as above.
- **The owner's manual Fedora check.**
- **A refused UID through the running binary.** It needs a second account. `reviewer-1` reasons that case from the shared refusal arm and does not reproduce it.
- **Step-by-step replays.** Neither the step-5.1 mutation (`is_socket()` removed) nor the step-5.5 mutation that turns the refusal arm into `return Ok(())` was replayed. Both are accepted from the journal.
- **`bridge.py`.** Outside the plan's scope. Only `request_host` was read, as evidence for `reviewer-1`, and the suite was run.
- **Lenses that belong to other passes:** cost and machinery (`CONTRIBUTING.md` Part II), cross-module coherence, and commit hygiene, the attribution trailers included.

**Conforming, stated so that silence is not read as unreviewed.**

- **`D-11`.** Each required property holds as written. This was read in the source and, where noted, probed.
  - `wake_on_termination` reads each signal's inherited action and skips `SIG_IGN`.
  - The handler only writes to the pipe.
  - Handlers are installed with no `SA_RESTART`, and `accept` has no `Interrupted` arm.
  - `Bridge` holds `_watcher` and then `Socket`. It is moved into the scope closure, so it drops before any join.
  - `Socket::drop` removes the path, and only then does the listener close.
  - `lock` is dropped after `thread::scope` returns.
  - The request phase alone waits on the wake pipe, and bytes already sent are still read.
  - `poll` callers retry `Interrupted`. `accept` retries `WouldBlock` and `ConnectionAborted`.

  The doc comment's account of `EINTR` matches the blocking calls left in the serving process, checked by reading the `std` paths those calls take.
- **`D-12`.** `not a UID: <value>`. Both table tests run through `Scratch::run`, the usage table has a `handle-stdio` row, and the separate test is gone.
- **`D-4` §Remediated.** The writer thread is gone. Stdin is non-blocking and written inside the `poll` loop through `Poll::optional`. A failed write is still reported only on success. Parity with the earlier `capture`, which joined a thread after a successful tool, was checked on each exit path.
- **`D-8` §Remediated.** The `clipboard::watch` comment states both endings truthfully. The tools a sync starts run in groups of their own and are not killed with the watcher's group. That is consistent with the comment's "truncated selection".
- **`F-12`.** `README.md` §Tests and limitations and `test-architecture.md` §The layers state the UID check and the phase deadlines as proved at the function level. The latter has a change-record entry.
- **`F-15`.** `Scratch::run` and `Scratch::serve` are bounded by `PATIENCE` and kill on drop.
- **`F-20`.** The comment at the refusal site holds. One short line into a freshly accepted stream's empty send buffer does not wait.
- **`F-22`.** `Deadline`'s comment gives the adjudicated premise.
- **`F-23`.** The alias has no `Send + Sync`.
- **The waived arms of `F-14` §Ruling** are named as waived in the journal and in `D-11` §Implemented.
- **The concurrency test's retry is sound for what the test claims.**
  - The seventeenth connection's refusal is asserted outside the loop. That peer writes nothing, so its close carries nothing unread.
  - The loop exits only on `ok:true`. A parsed non-`ok` header must be the limit refusal.
  - Only an exchange that failed at the I/O level is retried, and only until `PATIENCE`.
  - A regression that never frees a place, or that resets every connection, still fails the test with `no place came free`.

  The retry's comment is `reviewer-3`.

The exceptions follow.

## Findings

Tier counts: A 1 · B 1 · C 1.

### Finding reviewer-1 — Tier A — A refusal is lost to a client that sends before it reads, the shim included

- **Finding.** In each of these cases, `serve` writes an error line and drops the stream without reading what the peer sent:
  - a refused UID;
  - the concurrency limit;
  - a failure to read peer credentials;
  - a failure to make the stream non-blocking;
  - a worker that cannot start;
  - a worker that turns away an unfinished request at shutdown.

  Once the bridge's end is closed, the peer's next write fails with `EPIPE`. If the peer's data was still unread at the close, the peer's next read after the queued bytes fails with `ECONNRESET`. `bridge.py`'s `request_host` calls `sendall` before it reads. When the bridge's close lands before that `sendall`, the shim raises `BrokenPipeError`, so it never reads the refusal already waiting in its receive queue. The user sees `wl-paste: [Errno 32] Broken pipe`.
- **Current behaviour / contract.**
  - `D-11` covers a failure that concerns one connection: "Each gets an error response where one can still be written". For a peer still sending its request, it says "it gets an error response, `bridge is shutting down`".
  - `README.md` §Access errors: "the error names the UID the host sees … Use the UID from the error".

  The initial implementation on `main` refuses the same way, writing the refusal and then `continue`. That was read from its `serve`, not run. The behaviour predates the branch, and phase 5 did not change it.
- **Why it is a problem.** A container whose UID is refused gets the remedy that README sends the user to look for only part of the time. The rest of the time it gets an error that names nothing. The same is true of `too many concurrent clipboard requests`. The journal's phase-5 note records the mechanism differently: "the peer's read can fail with `ConnectionReset` before the refusal is read … Observed only through the concurrency test, under parallel load". The probe below shows three things the note does not:
  - the refusal's bytes arrive every time;
  - the shim loses them on its write, not on its read;
  - it happens without load.

  The owner would size the item from that note.
- **Evidence.** Python probes against the reviewed state's debug binary, with a stand-in `wl-paste` and the socket in a scratch directory. Sixteen idle connections held every place, and then attempts ran one at a time:
  - **The shim's own sequence** (connect, `sendall` the request, `makefile("rb").readline`): 109 of 200 attempts got `too many concurrent clipboard requests`, and 91 of 200 got `BrokenPipeError` from `sendall`.
  - **The same, with 50 ms between connect and send:** 20 of 20 got `BrokenPipeError`.
  - **Send, then read to EOF**, the concurrency test's exchange: 114 of 200 got `BrokenPipeError` on the send. In the other 86, the complete refusal header was received, and the next read raised `ConnectionResetError`. In no attempt did a reset arrive before the refusal's bytes.
  - **Shutdown.** A peer sent `{"op":`, and the bridge got SIGTERM. When the peer sent the rest 200 ms later, it got `BrokenPipeError`. A `recv` after that returned `{"error":"bridge is shutting down","ok":false,"size":0}`.
  - **A refused UID** goes through the same arm of the accept loop in `server::serve`, the `write_response(Deadline::new(&stream, PHASE), Err(error)); continue` that the concurrency refusal takes. It was not run.
- **Routing.** Architect. Two questions are contract calls. Does `D-11`'s "gets an error response" require delivery to a peer that sends before it reads? Which side owns that property: the host's close, or the shim's handling of a failed send? `bridge.py` is outside this plan's scope.
- **Required property.** A container user whose connection the bridge refuses sees the refusal's message, whatever the order of the peer's send and the bridge's close. Otherwise the record states that a refusal may surface as a broken connection.

### Finding reviewer-2 — Tier B — The termination-signal binary test fails when the runner was started with SIGINT or SIGHUP ignored

- **Finding.** Step 5.3 makes `serve` keep a disposition it inherited as ignored. `should_remove_the_socket_and_exit_0_on_a_termination_signal` sends SIGTERM, SIGINT and SIGHUP in turn to a `serve` that `Scratch::serve` starts. That `serve` inherits the runner's dispositions, because `std::process::Command` restores none except SIGPIPE. Suppose the runner has SIGINT or SIGHUP ignored. Then `serve` correctly keeps running, as `D-11` requires. The test fails after `PATIENCE` with `serve did not exit`, which names neither the signal nor the precondition. The test's own `"SIG{signal}"` message is never reached, because the panic is raised inside `Server::wait`. Any binary test that relies on SIGTERM has the same dependency, but SIGTERM is rarely inherited as ignored.
- **Current behaviour / contract.**
  - `CONTRIBUTING.md` §Tests: a test needing what the default run cannot start is ignored, "so `cargo test` stays correct on a machine with nothing provisioned".
  - `test-architecture.md` §The layers says of the binary tests: "They need only `/bin/sh` and coreutils, so they run in the default `cargo test`".
  - `D-2`: they "run in the default `cargo test` on a fresh checkout".
- **Why it is a problem.** The binary layer is the repository's instrument for the lifecycle. In these environments it reports a server defect where `serve` meets its contract. No unusual setup is needed: a POSIX shell ignores SIGINT for a background job of a non-interactive script, so a script that runs `cargo test &` is enough, and `nohup cargo test` does the same for SIGHUP. The journal records the assumption under "Noticed, not acted on". No register entry carries it.
- **Evidence.** Each run used the reviewed state:
  - `sh -c "trap '' HUP; exec cargo test --test binary should_remove_the_socket_and_exit_0_on_a_termination_signal"` failed in 10.03 s, panicking in `support.rs` with `serve did not exit`.
  - The same with `trap '' INT` failed in 10.02 s.
  - `sh -c "cargo test --test binary should_remove_the_socket_and_exit_0_on_a_termination_signal & wait"` failed in 10.03 s.
  - With default dispositions, the test passes.
- **Routing.** Architect. How a binary test may hold a signal's disposition bears on `D-2`'s constraints: `/bin/sh` and coreutils only, and no dev-dependency. It also bears on the workspace lint table, which denies `unsafe_code`. A POSIX shell cannot reset a signal that was ignored when it started.
- **Required property.** The binary layer's verdict on signal handling does not depend on the dispositions its runner inherited. Where the precondition does not hold, the failure says so rather than reporting a server defect.

### Finding reviewer-3 — Tier C — The concurrency test's comment gives the wrong mechanism for the exchange it retries

- **Finding.** In `should_refuse_a_connection_over_the_limit_and_keep_serving` (`crates/host/tests/binary/server.rs`), the retry is justified by this comment: "A refused connection is closed with the request unread, which can reset it before the refusal is read, so a failed exchange is one more attempt."
- **Current behaviour / contract.** In every attempt that reached its read, the refusal's bytes were received. The exchange fails in one of two ways:
  - on `write_all`, with `EPIPE`, when the bridge's close came first;
  - on the read after the refusal, with `ECONNRESET`. `read_to_end` then returns an error, so the loop never looks at the refusal already in `bytes`.

  `CONTRIBUTING.md` §Comments: comments describe the code that exists now.
- **Why it is a problem.** The comment, and the journal note it mirrors, tell a reader that the refusal is lost on the read. That points a reader of `reviewer-1` at the wrong half of the exchange. The effect is internal only.
- **Evidence.** The send-then-read-to-EOF probe in `reviewer-1`: 114 of 200 attempts got `BrokenPipeError` on the send, and 86 of 200 got the full refusal header followed by `ConnectionResetError`. None got a reset before the refusal's bytes.
- **Routing.** Implementer.
- **Required property.** The comment states the failure that the retry absorbs, as that failure occurs.
