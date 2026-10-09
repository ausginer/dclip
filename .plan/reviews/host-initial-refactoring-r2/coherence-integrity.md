# Crate coherence: integrity pass, round host-initial-refactoring-r2

## State read

Round `host-initial-refactoring-r2`: the branch `host/initial-refactoring` after phase 5 of the plan (the remediation of round `host-initial-refactoring-r1`), read against the register as it stands after the phase-5 settlement commit. Read in full: `.plan/00-index.md` (every entry for `D-1`–`D-13`, `F-1`–`F-24`), `.plan/host/initial-refactoring/{plan,journal,decisions,analysis,README}.md`, the r1 integrity report; `crates/host/src/{server,process,sys,cli,clipboard,main,protocol}.rs` and the unit test files that changed; `crates/host/tests/binary/{server,cli,support}.rs`; `bridge.py`, `test_bridge.py`, `setup-host.sh`, `crates/host/Dockerfile`; `README.md`, `CONTRIBUTING.md`, `.agents/docs/{test-architecture,documentation,handoff,review-findings}.md`. `Q-1` was not reviewed, as instructed.

## Scope

**Covered**

- The protocol boundary, both ends: request line, response header, framing, error strings, bounds and timeouts, `bridge.py` against `server.rs` and `protocol.rs`. The new strings `bridge is shutting down` and `not a UID: …` were followed to where each reaches (the socket, and stderr respectively).
- The trust boundary: admission order (peer UID, then concurrency) before any read; request and response bounds; the new shutdown stop on the request phase; tool reaping on the rewritten `capture` loop; absence of any clipboard write or non-image read on the request path.
- Neighbouring flows touched by phase 5: shutdown ordering (watcher, socket path, listener, workers, lock), inherited signal dispositions, `EINTR` handling after `SA_RESTART` was dropped, text sync's X11 write through the non-blocking stdin feed, the CLI's error text.
- Instruments: the binary and unit tests, and the register and README coverage claims, checked by mutation in a scratch copy (outside the checkout).
- The three journal items: the refusal lost to a reset, the concurrency test's retry, and the dependence of the termination-signal tests on the runner's signal dispositions.

**Evidence gathered**

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (37 unit, 19 binary) are green at the launch state; the same suites pass for `--target x86_64-unknown-linux-musl`; the Python suite passes (4 tests). The release musl binary is 611,072 bytes, matching the journal.
- Every test name that the register, `README.md` and `.agents/docs/` cite exists in the tree, except `should_refuse_handle_stdio_as_an_unpublished_invocation` in `D-9` §Implemented. `D-9` is superseded and `D-12` §Implemented names the replacement, so this is not a finding.

**Not covered**

- The Docker image build (no Docker CLI). Whether the builder's `RUN` environment hands the tests default signal dispositions is therefore unverified; see `integrity-3`.
- The owner's end-to-end check on Fedora with a real compositor.
- The correctness of each step against its own decision (another pass's subject), except where a finding below needs it.
- Orphaning of the tools a killed `sync-text` itself spawned. Each tool runs in its own process group, so the shutdown's group kill reaches the sync but not its `wl-paste` or `xsel`. This is the same fact `clipboard::watch`'s comment states for `xsel`; it was examined and not shown to differ from what the record says, so it is not a finding.

## Null results

- **Trust boundary: no bypass found.** Every accepted connection still goes through `admit` before a byte is read. The request is bounded by `take(4097)` under a total deadline; the new shutdown stop only shortens a wait. The response phase has no stop and keeps its own 12 s bound. The payload is bounded by `LIMIT` in `capture` and by the magic check. No path writes the clipboard from a request. The stdin feed closes its pipe end on every path out of `capture`, and an unreaped tool is still killed by group on drop.
- **Protocol boundary: no format drift.** Request language, header fields, framing, the type set (`IMAGE_TYPES` against `image::Format::ALL`) and the size bound (`MAX_BYTES` against `process::LIMIT`) are unchanged. The shim prints any `error` string it is given, so `bridge is shutting down` needs no shim change. The one boundary defect found concerns delivery, not format (`integrity-1`).
- **Public surface: no addition.** No flag, subcommand or environment variable was added. `handle-stdio` is refused as an unpublished invocation; a malformed `serve` argument now prints `not a UID: <value>` where it printed `invalid digit found in string`, which is the change `D-12` requires.
- **`EINTR` after `SA_RESTART` was dropped.** The sites that can block in the serving process are `poll` (retried by every caller), `Child::wait` and the thread join (both retried inside `std`), and non-blocking socket and pipe calls, which never sleep. I found no blocking call that a handler can now fail. The comment in `sys::wake_on_termination` is accurate.
- **Inherited ignored signals.** A bridge started with SIGHUP ignored survives SIGHUP and still answers (binary test, and by hand). `README.md` step 2 states it.
- **`capture` rewrite.** Input larger than a pipe is fed while the output is read; an empty input closes stdin at once; a tool that exits without reading is reported only when it exits successfully, as before; no iteration of the loop can poll an empty descriptor set and sleep to the deadline.
- **Clean-up order as built.** The watcher stops, the path is removed and the listener closes before any worker is joined; the lock is released after the join. The code does what `D-11` says. `integrity-2` is about whether anything would notice if it did not.

## Findings

### integrity-1 — A refusal, and the shutdown turn-away, usually do not reach the shim's user under load: the container sees `Broken pipe`

**Tier A**

- **Current behavior / contract.**
  - `README.md` §Access errors: when the container's UID is refused, "the error names the UID the host sees", so that the user can pass `--allow-uid`. `CONTRIBUTING.md` (the paragraph on shipped messages) relies on the same: the one-clause remedy in the message is "read inside the container, where the README is not".
  - `D-11` has a connection that is still sending turned away with `bridge is shutting down`.
  - `server::serve` writes a refusal on the accept thread and drops the stream at once, with the request unread.
  - `bridge.py::request_host` connects, calls `sendall` on its request, and only then reads. A `sendall` failure is an `OSError`, which `main` prints as `wl-paste: <error>`.
- **Why it is a problem.** When the host has refused and closed before the shim's `sendall`, the shim's send fails with `EPIPE` and the shim never reads the refusal that is waiting in its receive queue. The container sees `wl-paste: [Errno 32] Broken pipe`. The remedy, the UID, is lost, which is the one message the README sends users to. The journal describes this as the refusal lost to a reset and says the shim "may" see a reset. What the shim sees is different: the reset (`ECONNRESET` after unread data is closed) is delivered after the refusal's bytes, so a reader that stops at the header line, as the shim does, still gets the message. The loss is on the send side. The concurrency test's retry loop absorbs both shapes, and its first, deterministic assertion connects and sends nothing, which is the one shape that cannot lose the message. No test exercises a refusal for a peer that sends, which is every real peer.
- **Evidence.** Against the release binary from this state, with a stand-in `wl-paste` on the host and sixteen idle connections holding every place, `bridge.py` was run as `wl-paste -l` repeatedly (the refusal path for the concurrency limit is the one `admit` shares with the UID check):
  - On an idle machine: 118 refusals, 1 of them `wl-paste: [Errno 32] Broken pipe`, the rest `too many concurrent clipboard requests`.
  - With 24 busy loops competing for the 12 cores: 39 refusals, 22 of them `Broken pipe`.
  - A raw client that connects, pauses 2 ms and then sends: 100 of 100 sends raised `BrokenPipeError`, and in every case a following `recv` returned the full refusal JSON. The message was delivered and discarded.
  - The shutdown message has the same shape: a client connected before SIGTERM that sends afterwards gets `BrokenPipeError` on send and `{"error":"bridge is shutting down",…}` on `recv`.
  - The behaviour is the initial implementation's too (it refused the same way). It is reported because the round's scope names it and because no entry in the register carries it; only the journal's "noticed, not acted on" does.
- **Required property.** A peer that is refused, or turned away at shutdown, learns why, whether its request line reaches the host before or after the host closes. Whether that property is owed by the host (not closing on an unread request), by the shim (reading the response after a failed send) or by both is a call about the protocol boundary, so it is routed. A test that sends a request to a refusing host and reads the message, through the shim's own code path, is what would show it holds.

### integrity-2 — Two ordering properties of `D-11` have no witness: the bridge stops advertising before the join, and the lock is released after it

**Tier B**

- **Current behavior / contract.**
  - `D-11` requires that "the bridge stops advertising before any worker is joined" and that "the lock is released last, after every worker has finished. A successor started during the drain fails on the lock".
  - `D-11` §Implemented and `F-11`'s status (Settled by `D-11` §Implemented) rest on the listed witnesses. `README.md` §Tests and limitations lists the clean exit "that removes the socket at once, turns away a client still sending its request and waits for requests already received" among what the binary tests cover.
  - `server::serve` implements the ordering with `let bridge = bridge;` inside the scope closure (`server.rs:242`) and `drop(lock)` after the scope (`server.rs:308`).
- **Why it is a problem.** The one witness for the first property is `should_unadvertise_at_once_and_turn_away_an_idle_peer_on_a_signal`, and it passes whatever the ordering. The idle peer's worker now leaves its wait at shutdown, so the join is milliseconds long and the socket path disappears within the test's 1 s whether it is removed before the join or after. The only worker that makes the join long is one running a tool, and the one test with such a worker (`should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal`) does not look at the path or at a new connection. The second property has no test at all. A reordering is what `F-11` was about, and nothing would fail on its return.
- **Evidence.** Mutations in a scratch copy, each followed by the full binary suite (19 tests):
  - `bridge` taken back out of the closure and dropped after the scope, before the lock: 19 passed.
  - the lock moved into the closure, so it is released before the join: 19 passed.
  - For contrast, releasing the lock right after the bind fails `should_refuse_a_second_instance_while_the_first_holds_the_lock`, so that test pins "lock outlives the loop" and nothing more.
- **Required property.** Each ordering property the register states for shutdown can be falsified by a test that fails when the order is reversed, or the entry says which ones are held by construction only. The natural shape is the one slow request that already exists: with a tool running, after the signal the path is gone and a new connect fails at once, while a second `serve` still fails on the lock until the request finishes.

### integrity-3 — The termination-signal binary tests fail, with a message that does not say why, whenever the runner started with SIGHUP or SIGINT ignored

**Tier B**

- **Current behavior / contract.** `should_remove_the_socket_and_exit_0_on_a_termination_signal` starts `serve` through `Scratch::serve` and sends it TERM, INT and HUP in turn. Since `D-11`, `serve` keeps a disposition it inherited as ignored, and `Scratch::serve` passes the test process's own dispositions down unchanged. `handoff.md` and `test-architecture.md` require the default `cargo test` to be correct "on a machine with nothing provisioned"; nothing in `test-architecture.md`, `README.md` or the test says the test needs default dispositions.
- **Why it is a problem.** The test's outcome depends on how its runner was launched, which the test neither sets nor checks. A runner under `nohup`, a background job of a non-interactive shell (SIGINT), or any CI wrapper that sets either, fails the suite with `serve did not exit` after ten seconds. A reader of that message cannot tell a regression in `serve` from the environment. The Docker build runs this suite, and whether its `RUN` environment is clean is unverified here.
- **Evidence.** In a subshell with `trap '' HUP` (and then `trap '' INT`), running `cargo test --test binary` directly: 18 passed, 1 failed, `server::should_remove_the_socket_and_exit_0_on_a_termination_signal`, `serve did not exit` at `tests/binary/support.rs:249`. With default dispositions: 19 passed. (`timeout` resets the dispositions before running its child, so a probe through `timeout` misses it.) The journal records the same assumption as "noticed, not acted on".
- **Required property.** The result of the binary suite does not depend on the dispositions the runner inherited: the test that expects a handled signal gives `serve` default dispositions for the signals it sends, or says in its failure that an inherited ignore is the cause. If the suite is meant to need clean dispositions, `test-architecture.md` says so where it lists what the default run needs.

### integrity-4 — Only SIGHUP's inheritance is witnessed, while the rule, the README's coverage list and the README's launch advice speak of all three

**Tier C**

- **Current behavior / contract.** `D-11`: "Inherited ignored signals stay ignored", for SIGTERM, SIGINT and SIGHUP alike; the architect's judgement call (journal) was to extend it to SIGINT and SIGTERM. `README.md` §Tests and limitations lists the tests as covering "a signal the bridge was started with ignored stays ignored". `README.md` step 2 says "closing it, Ctrl+C or SIGTERM stops the bridge" and gives SIGHUP as the only inherited exception.
- **Why it is a problem.** The one test, `should_keep_a_signal_ignored_when_serve_inherits_it_ignored`, ignores HUP. A mutation that honours the inherited ignore for SIGHUP only survives. And a user whose launcher ignores SIGINT (a shell that starts a job without job control) is told Ctrl+C stops the bridge. `D-11`'s own witness list names only SIGHUP, so no register cell is false, which is why this is C.
- **Evidence.** In a scratch copy, `if signal == libc::SIGHUP && inherited.sa_sigaction == libc::SIG_IGN` in `sys::wake_on_termination` (`sys.rs:60`): 19 binary tests passed.
- **Required property.** The README's account of when the bridge keeps running agrees with `D-11` for each signal, and a coverage sentence that names no signal is one the tests back for each signal it covers, or names the one signal tested.

### integrity-5 — The folder README still says phase 5 is next

**Tier C**

- **Current behavior / contract.** `.plan/host/initial-refactoring/README.md` Status: "Next: phase 5 of the plan, the remediation, for an implementer. After that come a second review round and the owner's end-to-end check on Fedora."
- **Why it is a problem.** The journal's 2026-10-09 entry and the register (`D-11`, `D-12`, `D-4`, `D-8` remediated, `F-10`–`F-24` settled) record phase 5 as done and the second review round as the next step. A reader starting from the folder README is pointed at work that is finished.
- **Evidence.** The Status line above, against the journal entry "Phase 5: remediation of round `host-initial-refactoring-r1`" and its **Waiting** list.
- **Required property.** The folder README's status names the state the journal and register record.

## Journal items, read together

- **The refusal lost to a reset.** Real, and wider than the journal says; its effect on the shim is `Broken pipe` and not a reset. `integrity-1`.
- **The concurrency test's retry workaround.** Sound for what it protects: it ends on a served request, fails after ten seconds otherwise, and the refusal text is asserted exactly when a refusal is read. It does not pin delivery of a refusal to a peer that sends, and the register's `F-14` status ("Settled by `should_refuse_a_connection_over_the_limit_and_keep_serving`") rests on the refusal arm and the limit, which it does pin: the `admit` mutation and a `return` in the refusal arm fail it, per the journal, and the first assertion is deterministic. No finding beyond `integrity-1`.
- **The signal tests' assumption.** Confirmed by running it. `integrity-3`.

## Tier counts

A: 1 (`integrity-1`). B: 2 (`integrity-2`, `integrity-3`). C: 2 (`integrity-4`, `integrity-5`).
