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

**A phase-2 defect, found and fixed in 3.4.** In phase 2, `capture` marked success by assigning `tool = Tool::Reaped`. Assigning to a value with a `Drop` drops the old value, so the success path killed the tool's process group: the `xsel -i` selection daemon would have been killed after every sync. No test saw it, because no stand-in forked a survivor. The first version of the 3.4 fix repeated the mistake inside `try_reap`, and `should_not_signal_the_group_of_a_reaped_child` caught it. `Drop` now lives on `Tool`, a wrapper around `State`, so a state change drops only the `Child` handle. `should_leave_the_group_of_a_successful_tool_alone` pins the success path. The defect existed only in commits `b117205`–`b9b6686` on this branch.

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
