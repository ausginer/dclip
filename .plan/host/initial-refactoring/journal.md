# Journal

A dated log of the work on this refactoring: what was done, what was found, what is waiting. Newest last. Entries are append-only.

## 2026-10-06 — Analysis and decisions (architect)

**Session.** Architect role on branch `host/initial-refactoring`. The first attempt ran without the effort guard (`Effort guard active` was absent), so work stopped. The owner registered the marketplace and restarted, and the guard then recorded `declared=high, actual=high, match, enforced` for this session's tool calls.

**Read.**

- `crates/host/src/main.rs` in full, plus both manifests, the `Dockerfile` and `.dockerignore`.
- `bridge.py`, `setup-host.sh` and `README.md`.
- `CONTRIBUTING.md` in full, because the request was a whole-crate refactoring, which every Part II section bears on.
- `.agents/docs/test-architecture.md`, `documentation.md`, `review-findings.md` and `handoff.md`.

**Baseline** (`I-1`):

- 13 tests passing; clippy and fmt clean.
- Release musl binary: 619,264 bytes.
- Dependencies: `libc` and `serde_json`.

**Probes** (`I-1`, `I-2`, `I-3`, methods in [`analysis.md`](analysis.md) §Measurements):

- Capture throughput is about 14 MiB/s. A 63 MiB capture misses its 4 s bound, which makes `F-1` the most consequential finding.
- A slow peer held a connection for 30 s and counting (`F-2`).
- The watcher survived SIGKILL of the bridge (`F-3`).
- A non-UTF-8 listing line breaks reads (`F-4`).
- The response header costs 14 syscalls (`F-5`).
- A child inherits the parent's blocked signal mask through `Command`. This contradicted the expectation going in, that `std` resets the mask, and it changed `D-7` from `signalfd` to a wake pipe.
- `serde` derive costs 28 KiB and 2.4 s of clean build, and does not refuse unknown fields on unit variants (`I-3`).

All probes ran in scratchpad copies. Nothing was added to the tree to take them.

**Minted:** `D-1`–`D-10`, `F-1`–`F-9`, `Q-1`, `I-1`–`I-3`, in [`00-index.md`](../../00-index.md), which this change creates.

**Where the record lives.** The request was for decisions, journal and the rest in this directory. The repository's record convention requires every entry to be claimed exactly once, in the register (`documentation.md` §6, `review-findings.md` §Artifacts). So the entries — each decision's statement, argument and required properties — are in `00-index.md`. This directory holds the analysis, the rejected alternatives, the plan and this journal, and names entries without claiming them.

**Judgement calls an owner may want to revisit:**

- "Separate tests" was read as both sibling test files and a new binary-level layer (`D-2`), which amends `CONTRIBUTING.md` §Tests. The amendment lands with the layer in phase 1, not in this change. A current-state rule should not describe tests that do not exist yet.
- SIGHUP joins the shutdown signals (`D-7`). That is a small behaviour change beyond fixing a finding.
- Worst-case shutdown with a slow peer in flight is about 32 s once workers are joined (`D-7`). Interrupting them was deferred.
- `handle-stdio` is deleted rather than documented (`D-9`).

**Waiting:**

- `Q-1`, for the owner.
- Phase 1 of [`plan.md`](plan.md), for an implementer.

## 2026-10-06 — Phase 1: the binary layer pins current behaviour (implementer)

**Session.** Implementer role on branch `host/initial-refactoring`; `Effort guard active` was present at start.

**Done.**

- `crates/host/tests/binary/` is one test target (`main.rs` plus modules), so the layer links once. `support.rs` holds the scratch directory under the system temporary directory, stand-in tools, `serve` started and waited for through its banner line, and a client that splits header and payload at the first newline without the host's framing code.
- Thirteen characterisation tests, one per item of the plan's phase-1 list, all passing against the unmodified source. Two items are tables of inputs exercising one behaviour: malformed `serve` arguments, and SIGTERM and SIGINT.
- `Dockerfile` and `.dockerignore` admit `crates/host/tests`.
- `D-2`'s amendment for the binary layer: `CONTRIBUTING.md` §Tests and `test-architecture.md` §The layers and §Failure interpretation, each with a change-record entry carrying the replaced wording. The sibling-file half of the amendment lands in phase 2 with the files it describes, so that neither document describes tests that do not exist yet.

**Not done.** The Docker CLI is not installed in this devcontainer, so the image build was not run and whether it runs the new layer is unconfirmed. `cargo test --locked --target x86_64-unknown-linux-musl`, the command the image runs, passes here.

**Measured.** Release musl binary 619,264 bytes, unchanged from `I-1`, as expected with no source change.

## 2026-10-06 — Phase 2: restructure with no change in behaviour (implementer)

**Done**, in two commits rather than the plan's seven steps, because steps 1–6 rewrite the same lines and an intermediate split without types would have been written only to be replaced:

- Workspace `[lints]`: `unsafe_code`, `clippy::undocumented_unsafe_blocks` and `clippy::unwrap_used` at deny; `rust-version` 1.89; `clippy.toml` sets `allow-unwrap-in-tests`. That setting reaches `#[test]` functions only, so `tests/binary/support.rs` carries a narrow `allow` with its reason.
- Modules as `D-1` lists them — `cli`, `image`, `protocol`, `clipboard`, `process`, `server`, `sync`, `sys` — with tests in sibling `tests.rs` files. `rg -n unsafe crates/host/src` finds nothing outside `sys.rs`. `flock` is `File::try_lock`.
- `D-3` types: `Format` (declaration order is preference order, pinned by a test), `Request` parsed by hand from `Value`, `Query` as the clipboard seam, `cli::Command` and `ServeOptions`. The unit tests changed only their mock.
- `Tool { Unreaped(Child), Reaped }` replaces `ChildGuard(Child, bool)`. As the plan asks, capture still marks it `Reaped` only on success, so a child that exited non-zero is still signalled; phase 3 makes the variant tell the truth, with the test that shows the difference.
- `D-10`: `sync::sync_text` takes the state and three tool closures. Unit tests pin the X11-equal skip, a failed X11 read counting as different, the image appearing by the second listing (with the query order), the state filter, and that the write carries exactly the text read.
- `D-9`: `handle-stdio` deleted; a binary test pins it as a usage error. The `CONTRIBUTING.md` and `test-architecture.md` sibling-file amendment landed with the split, with its change-record entries.

**One ordering difference.** `D-3` parses the request once, at the boundary, so a `read` naming an unsupported or non-string `type` is refused before the clipboard is listed, not after. The refusal text is unchanged; what differs is which error wins when the listing would also have failed, and that `wl-paste` is not run for a request that cannot succeed.

**Measured.** Release musl binary: 619,264 bytes after the restructuring commit (no change), 611,072 after deleting `handle-stdio` (−8,192, −1.3%, within the ±2% the plan sets). The delta is the stdin/stdout-locked request path and its `Read`/`Write` instantiations; `cargo bloat` is not installed here, so the attribution is from what was removed rather than from a symbol listing.

## 2026-10-06 — Phase 3: the findings fixed, one decision at a time (implementer)

Each step began with a test that failed for the reason its finding predicts, then the fix, then green. One commit per step.

| Step | Failing test first, and how it failed | Fix |
| ---- | ------------------------------------- | --- |
| 3.1 `D-5` | Both non-UTF-8 listing tests: `Utf8Error` | `clipboard::lines` over bytes |
| 3.2 `D-6` | Counting writer: 15 writes for one response | Header in its own buffer, one `write_vectored` with the payload |
| 3.3 `D-6` | Trickled request and slowly read response: each ran the peer's full 3 s against a 200 ms budget | `server::Deadline` |
| 3.4 `D-4` | 64 MiB binary test: `wl-paste timed out`; reaped-child test: the group member was killed | Event-driven `capture`, `Tool` over `State` |
| 3.5 `D-7` | SIGHUP: the process died with no exit code and left the socket; in-flight test: `serve` exited with its tool still running | Wake pipe, `thread::scope`, `admit`, `Bridge` |
| 3.6 `D-8` | The stand-in watcher was still running 10 s after SIGKILL to `serve` | `PR_SET_PDEATHSIG` in `pre_exec` |

**A phase-2 defect, found and fixed in 3.4.** In phase 2, `capture` marked success by assigning `tool = Tool::Reaped`. Assigning to a value with a `Drop` drops the old value, so the success path killed the tool's process group: the `xsel -i` selection daemon would have been killed after every sync. No test saw it, because no stand-in forked a survivor. The first version of the 3.4 fix repeated the mistake inside `try_reap`, and `should_not_signal_the_group_of_a_reaped_child` caught it. `Drop` now lives on `Tool`, a wrapper around `State`, so a state change drops only the `Child` handle. `should_leave_the_group_of_a_successful_tool_alone` pins the success path. The defect existed only on this branch, from the phase-2 restructuring up to the fix of step 3.4, and never reached `main`.

> 2026-10-08 — The sentence above named its span by commit identifiers. It was replaced in place under `D-13`, and the identifiers are not carried.

**`D-6`: socket timeouts were not enough.** The first deadline set `SO_RCVTIMEO` or `SO_SNDTIMEO` to the remaining time before each call. The slow-reader test still ran for 3 s: inside one large `writev` on a Unix stream socket, the kernel takes the timeout afresh for each buffer it allocates, so a reader that keeps draining slowly never lets a single call time out. Accepted streams are now non-blocking, and `Deadline` waits in `poll` for the remaining time. The required properties are met as stated. The mechanism is a different spelling from the decision's "every blocking call is given the time that remains", and is recorded here rather than as a new decision.

**`F-6`'s per-connection paths have no failing-first test.** The paths that used to end `serve` were: `EINTR` between `poll` and `accept`, a failure of `set_*_timeout`, and a panic from `thread::spawn`. None can be provoked deterministically from a test. The fix is structural. The loop body has no `?` except on an `accept` error that is not about the connection. Per-connection failures go through `admit` or a refused spawn, and each is answered on the stream. The unit tests pin `admit`'s refusals and its release of a place.

**Capture throughput after `D-4`** (the `I-1` loop as a temporary `#[ignore]`d test, release profile, same devcontainer; not committed):

| Payload | Before (`I-1`) | After   |
| ------- | -------------- | ------- |
| 1 MiB   | 71 ms          | 1.7 ms  |
| 8 MiB   | 567 ms         | 7 ms    |
| 32 MiB  | 2.25 s         | 24 ms   |
| 63 MiB  | 4.41 s         | 38 ms   |
| 64 MiB  | timed out      | 38 ms   |

Two runs each. The figures are `head` writing into a 64 KiB pipe. `F_SETPIPE_SZ` was not tried, because nothing is left for it to fix.

**Measured.** Release musl binary 619,264 bytes. That is +8,192 bytes over phase 2's 611,072, and the same as the baseline. The added code is the poll-driven capture with its pidfd, the `Deadline` wrapper, the wake pipe and three handlers, scoped workers through `thread::Builder`, and `Arc` per connection. `cargo bloat` is not installed, so this is attribution by what was added, not a symbol listing. The normal dependency edges are still `libc` and `serde_json`.

## 2026-10-06 — Phase 4: close (implementer)

- `README.md` §Tests and limitations states what the layers cover, and that Linux 5.3 or later is required (for `pidfd_open`). The build table says Rust 1.89+. Step 2 says that closing the terminal stops the bridge cleanly.
- The register marks `D-1`–`D-10` implemented, each with a `§Implemented` sub-clause naming its sites and tests, and `F-1`–`F-9` settled.

**Final measurements against the baseline (`I-1`):**

- Binary: 619,264 bytes, unchanged.
- Tests: 50, up from 13. That is 33 unit and OS-boundary tests in sibling files, and 17 binary-level tests.
- Capture: 63 MiB in 38 ms, against 4.41 s.

The Python suite passes and is untouched.

**Waiting:**

- **Owner:** the manual end-to-end check on Fedora that `plan.md` §Phase 4 lists: README step 5, one large screenshot, and one `--sync-text` session.
- **Owner:** the review round, for a fresh `consolidator`.
- **Owner:** the Docker image build. It has not been run, because this devcontainer has no Docker CLI.
- `Q-1`, out of scope as asked.

## 2026-10-07 — Adjudication of the `D-6` mechanism and the `D-3` error order (architect)

**Session.** Architect role on branch `host/initial-refactoring`; `Effort guard active` was present at start. `Q-1` stayed out of scope, as asked.

**Asked.** Do the two departures the implementer recorded conform to their decisions, or do they need superseding decisions or remediation?

**Ruled.** Both conform. Neither needs a new decision or any remediation. Each ruling is a dated `§Adjudicated` sub-clause of its entry in [`00-index.md`](../../00-index.md).

- **`D-6`.** The required properties hold, and a change of mechanism changes none of them, so it is not a substantive amendment (`documentation.md` §6). The decision's argument assumed that per-call socket timeouts could bound a phase. A probe falsified that: one blocking 8 MiB `send` with `SO_SNDTIMEO` at 200 ms ran 6.30 s against a slow reader. The correction is in the entry. The watchdog rationale in [`decisions.md`](decisions.md) rested on the same premise; it is rewritten there, with a dated note carrying the old wording.
- **`D-3`.** Refusing an unsupported `type` before the listing follows from parsing once at the boundary. No property is broken: the accepted language, the refused set and every message string are unchanged. Error precedence between independent failures was never part of the contract. The shim cannot send such a request. The change does breach the letter of `D-1`'s "the restructuring commit changes no behaviour". It is accepted as recorded, with a pointer under `D-1` §Implemented.

**Checked.** `cargo test --workspace`: 33 and 17 passed, including both `D-6` deadline tests. Old and new request paths compared by reading `respond_with` in the initial implementation against `protocol::Request::parse` and `protocol::respond_with`.

**Noticed, not acted on.** Neither is part of the question, so both are left for the review round.

- **A commit identifier in this journal.** The phase-3 entry names a commit range, which `documentation.md` §10 forbids in a tracked file. That entry is append-only and is not this session's to rewrite.
- **`Deadline` on a blocking stream.** `server::Deadline` documents that its stream must be non-blocking. In `server::serve`, if `set_nonblocking` fails, the refusal is written through a `Deadline` over a blocking stream. A header of under 100 bytes, written to a freshly accepted stream, cannot block in practice.

**Waiting.** Unchanged from the phase-4 entry: the owner's Fedora check, the Docker build, the review round, and `Q-1`.

## 2026-10-08 — Rulings on round `host-initial-refactoring-r1` (architect)

**Session.** Architect role on branch `host/initial-refactoring`, with `Effort guard active` present at start. The scope was every finding the round routed to the architect, the owner or the record owner, with `F-10` and `F-11` first. `Q-1` stayed out of scope. The round summary and all four pass reports were read in full. The code behind each routed finding was read at the round's state, which confirms the mechanism each finding describes. The pass probes were not re-run.

**Ruled:**

| Finding        | Ruling                                                                                                                                                       | Recorded in                         |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------- |
| `F-10`         | A signal inherited as ignored stays ignored. This holds for all three signals, not only SIGHUP.                                                              | `D-11`, superseding `D-7`           |
| `F-11`         | When shutdown is requested, the watcher stops, the path is removed and the listener closes, all before any join. A peer still sending its request is told `bridge is shutting down`. Received requests are drained. | `D-11`                              |
| `F-21`         | `SA_RESTART` and the `Interrupted` arm on `accept` go. `F-6`'s `EINTR` sentence is withdrawn with a note, and so is its counterpart in `analysis.md`.        | `D-11`, `F-6`                       |
| `F-13`         | A defect against `D-4`, not a conflict inside it. Stdin is written in the capture's `poll` loop, so the writer thread goes.                                  | `D-4` §Adjudicated                  |
| `F-17`         | `D-8`'s last property covers death of the serving process only. A graceful shutdown kills in-flight syncs with the watcher's group, as `D-4` intends. Only the comment is owed. | `D-8` §Adjudicated                  |
| `F-18`, `F-24` | A malformed `serve` names the argument at fault, and the UID message becomes one that does. `handle-stdio` becomes a row of the usage table.                 | `D-12`, superseding `D-9`           |
| `F-14`         | A concurrency test through the binary is owed. The arms that cannot be provoked are waived by the architect.                                                 | `F-14` §Ruling                      |
| `F-16`         | `D-6`'s error header was spelled in an order the host never sent. Corrected in place, as a non-substantive edit, and likewise in `analysis.md`.              | `D-6`                               |
| `F-19`         | §10 governs. An identifier already in a file is replaced where it stands, and the note does not repeat it. Applied here, and to the quotes in two pass reports. | `D-13`, `documentation.md` §6, §9 and §10 |

`F-12`, `F-15`, `F-20`, `F-22` and `F-23` stay with the implementer, as routed. [`plan.md`](plan.md) §Phase 5 orders them together with the rulings above.

**Judgement calls an owner may want to revisit:**

- **Honouring SIG_IGN for SIGINT and SIGTERM as well as SIGHUP.** For SIGINT and SIGTERM this departs from the initial implementation.
- **Ending the request phase at shutdown, but not the response phase.** The worst case with a slow reader is still the tools' deadlines plus 12 s.
- **Accepting a possible truncated X11 selection** when a graceful shutdown lands on a sync that is writing more than a pipe buffer of text. This was reasoned and not reproduced.
- **Superseding `D-9` rather than reading it narrowly.** Its "any other invocation" could not be read to agree with `D-3`.

**Waiting:**

- [`plan.md`](plan.md) §Phase 5, for an implementer.
- Then a second review round, the owner's Fedora check, the Docker build, and `Q-1`.

## 2026-10-09 — Phase 5: remediation of round `host-initial-refactoring-r1` (implementer)

**Session.** Implementer role on branch `host/initial-refactoring`. `Effort guard active` was present at start. `Q-1` stayed out of scope, as asked. One commit per step, plus one for a test flake found at the exit gates (below).

| Step | Test first, and what it showed before the change | Change |
| ---- | ------------------------------------------------ | ------ |
| 5.1 `F-15` | In a scratch copy, with the `is_socket()` limb removed, `should_refuse_and_keep_a_regular_file_at_the_socket_path` hung until `timeout` killed it at 40 s. With the new support, the same mutation fails in 10 s with `["serve"] did not exit`. Nothing committed for this. | `Scratch::run` kills the binary and fails after `PATIENCE`. `Scratch::serve` reads the banner on a thread, with the same bound. Both go through one kill-on-drop guard. |
| 5.2 `D-11` (`F-21`) | None possible, as planned. | `SA_RESTART` and the `Interrupted` arm on `accept` removed. `sys::wake_on_termination` says who retries what. |
| 5.3 `D-11` (`F-10`) | `serve` started under `trap '' HUP` stopped on SIGHUP: `serve stopped on an ignored SIGHUP`. | Each handler is installed only where the inherited action is not `SIG_IGN`. |
| 5.4 `D-11` (`F-11`, `F-22`) | With one idle accepted connection, the socket was still there 1 s after SIGTERM. A first ordering of the same test showed the idle peer answered only after the full 12 s, with `timed out`. | `Bridge` moves into the scope and drops before the join, the lock drops after it, and the request phase waits on the wake pipe as well. `Deadline`'s comment gives the adjudicated premise. README step 2 and §Tests and limitations updated. |
| 5.5 `F-14`, `F-12` | Passes against the code, as planned. In a scratch copy it fails under `F-12`'s mutation, with `admit` replaced by an unconditional slot: the seventeenth connection got `timed out` after 12 s instead of the refusal. It also fails with the refusal arm turned into `return Ok(())`. | No production change. README §Tests and limitations and `test-architecture.md` §The layers now state the UID check and the connection deadlines as proved at the function level. The latter has a change-record entry. |
| 5.6 `D-4` (`F-13`) | `F-13`'s shape under a 200 ms deadline took 3.00 s. | Stdin is written non-blocking in the capture's `poll` loop, and the writer thread is gone. `sys::Poll::optional` lets one array watch a changing subset, and `Deadline` uses it too. |
| 5.7 `D-8` (`F-17`) | None; a comment. | The `clipboard::watch` doc comment covers both endings. |
| 5.8 `D-12` (`F-18`, `F-24`) | `--allow-uid alice` printed `invalid digit found in string`. | `not a UID: alice`. `handle-stdio` is a row of the usage table, and its separate test is deleted. |
| 5.9 `F-20`, `F-23` | None; a precondition and a bound. | The refusal site states why a blocking stream cannot block there. The `Result` alias drops `Send + Sync`, and the crate builds without them. `F-22` was done in 5.4, as the plan's note expects. |

**Waived, not tested** (`F-14` §Ruling): a failure to read peer credentials, a refused UID, a failure to make the stream non-blocking, and a failure to start a worker thread. `F-12`'s second mutation, raising `PHASE` to 3600 s, still survives the suite. That is the reason the coverage text now says the deadlines are proved at the function level.

**Beyond the plan's list, one test each:**

- `should_turn_away_a_request_still_arriving_at_shutdown` and `should_read_a_request_that_arrived_before_shutdown` pin `Deadline::or_until_shutdown` at the function level.
- `should_feed_input_larger_than_a_pipe_while_reading_the_output` pins the interleaving that the writer thread used to provide for free.

**A flake, found and fixed in the test.** The new concurrency test failed in 3 of 8 parallel runs, with `ConnectionReset` while reading the response in its retry loop. Its cause is pre-existing server behaviour, which is noted below. The loop now counts a failed exchange as one more attempt. Afterwards, 15 of 15 runs passed.

**Gates.**

- `cargo fmt` and clippy with `-D warnings` are clean.
- `cargo test --workspace` and `cargo test --locked --target x86_64-unknown-linux-musl` each pass: 37 unit and 19 binary tests.
- The Python suite passes and is untouched.
- `rg -n unsafe crates/host/src` finds nothing outside `sys.rs`.
- The normal dependency edges are still `libc` and `serde_json`.

**Measured.**

- **Binary.** The release musl binary is 611,072 bytes. That is −8,192 bytes (−1.3%) against 619,264, which is also this phase's starting figure, rebuilt from a scratch checkout. `cargo bloat` was installed into the devcontainer's `~/.cargo/bin` for this measurement, and the text section moved from 463,086 to 459,406 bytes (−3,680). The removed writer thread accounts for about 2.8 KiB: its spawn hooks, `Thread::new`, its result packet and its closure. `capture` itself is 2.2 KiB smaller. The accept loop's scope closure grew by about 1.7 KiB. The file size moves in pages, so it shows the text section's delta as two pages.
- **Tests.** 56, up from 50 at phase 4 and 13 at the baseline.

**Noticed, not acted on.** None of these is in this phase's scope.

- **A refusal can be lost to a reset.** `serve` writes a refusal and closes the stream without reading the request. If the peer has already sent the request, Linux resets the connection, and the peer's read can fail with `ConnectionReset` before the refusal is read. The shim sends its request before reading, so a refused UID, whose message carries the `--allow-uid` remedy, may reach the container as a reset instead. Observed only through the concurrency test, under parallel load. The initial implementation on `main` refuses the same way, so this predates the branch.
- **The termination-signal binary tests assume handled signals.** Since `D-11`, `should_remove_the_socket_and_exit_0_on_a_termination_signal` fails if the test runner itself was started with SIGHUP or SIGINT ignored, for example under `nohup cargo test`, because `serve` inherits that disposition and now keeps it.
- **One commit lacks the attribution trailers.** The step 5.1 commit was made without the trailers the branch's other commits carry. It was left as made rather than rewritten.

**Waiting:**

- A fresh `consolidator` for a second round, scoped to this phase.
- The owner's Fedora check and the Docker build. The Docker CLI is still absent here.
- `Q-1`.

## 2026-10-09 — Rulings on round `host-initial-refactoring-r2` (architect)

**Session.** Architect role on branch `host/initial-refactoring`. `Effort guard active` was present at start.

- **Scope.** `F-25`, `F-26` and `F-29`, which the round routed to the architect. `F-32` and `F-33`, settled here as the owner asked. A remediation handoff for `F-25`–`F-34`. A recheck of the tests and comments that the affected superseded decisions cite.
- **Out of scope.** `Q-1`, and CI packaging, which the owner keeps as a separate follow-up.
- **Read.** The round summary and all four pass reports in full, and the code behind each routed finding: `server::serve`, `sys::wake_on_termination`, `bridge.py`'s `request_host`, and the binary test support and server tests. The crate is unchanged since the round's reviewed state.

**Probed.** On scratch copies of the reviewed state, on the repository's devcontainer with 12 CPUs and kernel 7.2.7. No probe was retained.

- **`F-25`.** The real shim, against `serve` with sixteen held places, ran 150 refusals per configuration:
  - unchanged: 2 lost on an idle machine, and 62 with two busy loops per CPU;
  - changed to read the response after a failed send: none lost in either.
- **`F-29`.** A copy that retries an interrupted `poll` as a `poll`, and retries no `accept` error:
  - for each of SIGTERM, SIGINT and SIGHUP, 20 of 20 idle exits were clean, with status 0 and nothing on stderr;
  - it served a request after 900 peers had connected and gone away before being accepted;
  - all 19 binary tests passed.
- **`F-26`.** Under `trap '' HUP INT`, a copy that starts `serve` through `env --default-signal=TERM,INT,HUP` passed all 8 server binary tests. In the same runner, the reviewed state's termination test failed after 10.02 s. `env` here is GNU coreutils 9.7. The image builder is Debian bookworm, whose coreutils 9.1 has the option, added in 8.31.

**Ruled:**

| Finding        | Ruling                                                                                                                                                         | Recorded in                     |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------- |
| `F-25`         | The refusal is always delivered, and reading it is the client's job. The shim reads the response after a failed send. The host never waits on a refused peer, and is unchanged. | `D-14`                          |
| `F-26`         | Every `serve` a binary test starts gets default dispositions through `env --default-signal`, which `D-2`'s coreutils admits. The precondition is established, not merely reported. | `D-15`, superseding `D-11`      |
| `F-29`         | An interrupted `poll` is retried as a `poll`, so a signal no longer depends on an `accept` falling through. `accept` is called only on readiness, and none of its errors is retried. `F-6`'s `ECONNABORTED` example is withdrawn with a note, and so is its counterpart in `analysis.md`. | `D-15`, `F-6`                   |
| `F-27`, `F-30` | Each is owed a test, not a statement. They are listed among `D-15`'s witnesses.                                                                                | `D-15`                          |
| `F-28`         | Now that `F-25` is ruled, the test's exchange follows `D-14`'s client rule and absorbs no I/O error.                                                            | `D-15`                          |
| `F-34`         | `D-10` fixes the seam as a closure, not the type of its parameter. Borrowing is owed, and it decides no contract.                                               | `F-34` status                   |
| `F-32`         | The model comment now states the lock-after-path constraint, which `D-15` requires.                                                                            | `documentation.md` §5.1 and §9  |
| `F-33`         | The folder README's status names this state.                                                                                                                 | [`README.md`](README.md)        |

`F-31` stays with the implementer, as routed. [`plan.md`](plan.md) §Phase 6 orders everything owed.

**Rechecked.** The round's closing note asked for the tests and comments named by each superseded decision's §Implemented clause to be checked. They are below, read at the reviewed state.

- **`D-7`, superseded by `D-11`:**
  - **Gone:** `SA_RESTART`.
  - **True under `D-15`:**
    - the doc comment on `sys::wake_on_termination`, which says that `poll`'s callers retry `Interrupted`;
    - the field-order comment on `server::Bridge`;
    - `server::admit`;
    - the unit tests `should_refuse_a_peer_whose_uid_is_not_allowed`, `should_refuse_a_connection_while_every_place_is_taken` and `should_release_a_place_when_its_connection_is_done`. They pin messages and counting, and nothing in them rests on a retired fragment.
  - **`should_remove_the_socket_and_exit_0_on_a_termination_signal`.** It still expects `D-7`'s unconditional handling. That is `F-26`, step 6.1.
  - **`should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal`.** It holds. It has the same unstated dependency on the runner's SIGTERM, which step 6.1 removes, and it is the natural carrier for `F-27`.
- **`D-9`, superseded by `D-12`.** `handle-stdio` is absent from `cli::parse` and `main`. The test `D-9` names is deleted, and `D-12`'s usage table carries its row. `D-9`'s text still names the deleted test. That entry is inactive and append-only, so it stays.
- **`D-11`, superseded here by `D-15`:**
  - **Hold:**
    - the dispositions read in `sys::wake_on_termination`;
    - `Bridge` dropping inside the scope, with the lock dropped after it;
    - `Deadline::or_until_shutdown`, with the unit tests `should_turn_away_a_request_still_arriving_at_shutdown` and `should_read_a_request_that_arrived_before_shutdown`;
    - `should_unadvertise_at_once_and_turn_away_an_idle_peer_on_a_signal`. It holds, but it is not an ordering witness (`F-27`).
  - **Change:**
    - the `accept` arms, and their comment about a vanished connection (`F-29`, step 6.3);
    - the concurrency test's retry and its comment (`F-28`, step 6.6);
    - `should_keep_a_signal_ignored_when_serve_inherits_it_ignored` and `serve_ignoring`, together with the latter's `nohup` doc comment (`F-30`, `F-31`, step 6.2).
- **Outside the code**, the retired premises now carry notes or are owed changes:
  - `F-6`'s note and `analysis.md`, for `ECONNABORTED`, with notes added here;
  - `documentation.md` §5.1, for `D-4`'s retired stdin writer (`F-32`);
  - `README.md` step 2, which names SIGHUP alone (`F-30`, step 6.2).

**Judgement calls an owner may want to revisit:**

- **The shim owes refusal delivery, and the host does not.** A third-party client that sends before it reads, and stops at a failed send, can still miss a refusal. `D-14` states the client rule rather than defending against such a client.
- **No `accept` error is retried.** An error that no probe has produced would now end `serve` with status 1 rather than being retried. `F-21`'s ruling applies here: no guard against a failure the process has not been shown to meet.
- **The binary layer needs GNU `env`.** A non-GNU coreutils, such as busybox, would fail at the start of every `serve` test, with `env`'s own error in the failure.
- **`F-27` is owed tests rather than a statement.** The finding allowed either.

**Waiting:**

- [`plan.md`](plan.md) §Phase 6, for an implementer.
- Then a third review round, the owner's Fedora check, the Docker build, and `Q-1`.

## 2026-10-09 — Phase 6: remediation of round `host-initial-refactoring-r2` (implementer)

**Session.** Implementer role on branch `host/initial-refactoring`. `Effort guard active` was present at start. `Q-1` and CI packaging stayed out of scope, as asked. One commit per step, plus one for a flaw in a step 6.3 test that the step 6.6 runs exposed (below). Every mutation ran in a scratch copy outside the checkout, and none was retained.

| Step | Test first, and what it showed before the change | Change |
| ---- | ------------------------------------------------ | ------ |
| 6.1 `D-15` (`F-26`) | Under `sh -c "trap '' HUP INT; exec cargo test --test binary"`, `should_remove_the_socket_and_exit_0_on_a_termination_signal` failed after 10 s with `serve did not exit`. | `Scratch::command` starts every binary through `env --default-signal`, so the refusal and `sync-text` runs go through it as well. The same run then passed 19 of 19. `test-architecture.md` §The layers names GNU coreutils 8.31, and its change record carries the old sentence. |
| 6.2 `D-15` (`F-30`, `F-31`) | In a copy that keeps an inherited ignore for SIGHUP only, the previous suite passed 19 of 19. The new test failed on its SIGTERM row, and on its SIGINT row when that row ran alone. | One row per signal, each stopped by another handled signal. `serve_ignoring(signal)` passes the signal to `env --ignore-signal`, and no shell is involved. `README.md` step 2 names all three signals. |
| 6.3 `D-15` (`F-29`) | The idle-signal test now also asserts an empty stderr. With an interrupted `poll` falling through to `accept`, it failed with exit status 1, and so did three other signal tests. The new departed-peer test passed before and after. | `Interrupted` from `poll` is retried with `continue`, and the comment there says why. `listener.accept()?` replaces the retry arms. The `sys::wake_on_termination` doc comment needed no change. |
| 6.4 `D-15` (`F-27`) | `should_stop_advertising_and_keep_the_lock_while_draining_on_a_signal` passed against the code. With the bridge dropped after the scope, it failed with `the socket outlived the signal`. With the lock released at the end of the loop, before the join, its successor `serve` started and ran. No other test failed under either mutation. | No production change. `README.md` §Tests and limitations names the lock held through the drain. |
| 6.5 `D-14` (`F-25`) | The forced-order refusal test failed with `BrokenPipeError`. The no-response test passed with `BrokenPipeError`. | `request_host` reads the response after a failed send, and raises the failed send if no header parses. The text in `test-architecture.md` and `README.md` covers the Python layer's new reach. The former has a change-record entry. |
| 6.6 `F-28` | With `F-12`'s `admit` mutation, the test still failed: the seventeenth connection got `timed out` after 12 s. | The exchange reads by the framing through `support::read_framed`, retries only the limit's refusal, and panics on any I/O error, naming the send's outcome. |
| 6.7 `F-34` | None; a parameter type. | `set_x11_text` and its seam take `&[u8]`. |

**Parallel runs (6.6).** The test executable for the binary suite ran 8 at a time. There were three idle rounds and two rounds under two busy loops per CPU, 40 runs in all, and all 40 passed with 21 tests each. The concurrency test never failed. The first time through, 9 of 32 runs failed, all in the departed-peer test from 6.3. It queued thirty departed peers against sixteen places, so its own request could be refused. Reading to end of stream then met the reset that `D-14` describes. It now queues fifteen, so that its request never meets the limit, and that fix is a commit of its own. An earlier pair of rounds is not counted. A script error gave the test binary a filter that matched nothing, so those runs executed zero tests.

**`D-14` in the real shim.** A scratch probe ran 150 refusals of the real shim against the release `serve` with sixteen places held, with the held connections renewed before their 12 s phase ran out. The changed shim lost none, idle or under two busy loops per CPU. The unchanged shim lost none idle and 102 under load.

**Gates.**

- `cargo fmt` and clippy with `-D warnings` are clean.
- `cargo test --workspace` passes, and so does `cargo test --target x86_64-unknown-linux-musl`: 37 unit and 21 binary tests.
- The binary suite passes under `trap '' HUP INT`.
- The Python suite passes, with 6 tests.
- `rg -n unsafe crates/host/src` finds nothing outside `sys.rs`.
- The normal dependency edges are still `libc` and `serde_json`.

**Measured.**

- **Binary.** The release musl binary is 611,072 bytes, the same as phase 5's figure. The text section is also unchanged, at 459,406 bytes. The binary still differs from a fresh build of phase 5's state, so the build did see the change.
- **Attribution,** from `cargo bloat` against that build:
  - the accept loop's scope closure grew by 9 bytes, from 6,584 to 6,593;
  - `main` shrank by 16 bytes, from 6,282 to 6,266, where `sync_text` and `set_x11_text` are inlined;
  - function alignment absorbs the net −7 bytes.
- **Tests.** 58 Rust tests, up from 56 at phase 5, and 6 Python tests, up from 4.

**Judgement calls an owner may want to revisit:**

- **What a failed send hides.** After a failed send, the shim reports that failure for an end of stream, a reset, and a partial or malformed header line alike. The host writes no malformed header, so the last case only matters for a host the shim does not ship with.
- **A smaller departed-peer witness.** It uses fifteen peers, three ways of leaving, and SIGSTOP to hold them unaccepted. The ruling's probe used 900, but this witness has to stay below the limit to be deterministic. A larger count would need the concurrency test's retry.
- **The scope of the `env` wrapper.** Every binary invocation in the layer goes through `env`, not only `serve`, because one builder serves both. `sync-text` and the command-line refusals do not care.
- **The rule in `CONTRIBUTING.md` §Tests** still says the binary tests need `/bin/sh` and coreutils, without GNU or 8.31. The plan named only `test-architecture.md`, so it was left unchanged.

**Waiting:**

- A fresh `consolidator` for a third round, scoped to this phase.
- The owner's Fedora check and the Docker build. The Docker CLI is still absent here.
- `Q-1`.
