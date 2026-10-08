# Refactoring plan

The order in which `D-1`–`D-10`, and then `D-11` and `D-12`, land, what each phase must leave true, and how it is checked. The decisions are canonical in [`00-index.md`](../../00-index.md), and their required properties are the whole of what an implementer owes. This plan only sequences them.

**Owner of each phase:** an `implementer` session. **After phase 4:** a review round, run by a fresh `consolidator`, and a manual end-to-end check on Fedora by the owner.

## Principles of the sequence

1. **Pin behaviour before moving code.** The binary-level layer is written first, against the current implementation, so the restructuring in phase 2 is proved behaviour-preserving by tests that predate it.
2. **Restructure without fixing, then fix without restructuring.** Phase 2 changes no behaviour. Phase 3 changes behaviour one decision at a time, each with a test that fails before its fix. A reviewer can then tell a moved line from a changed one.
3. **Each phase ends green and committed.** The loop in [`handoff.md`](../../../.agents/docs/handoff.md) §Verify what you changed runs at the end of every phase: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, plus the Python suite whenever the protocol is touched. Commits are subject-only, as `handoff.md` describes; the branch is pushed once, after the last phase.

## Phase 1 — Pin the current behaviour

Decisions: `D-2`, the binary layer and the documentation amendment only.

- Create `crates/host/tests/` with a small support module:
  - a unique temporary directory per test, short enough for `sun_path`;
  - writing stand-in `wl-paste` and `xsel` scripts;
  - starting `serve` with a `PATH` and `CLAUDE_CLIPBOARD_SOCKET` of the test's choosing, waiting for the socket, and stopping the server on every path;
  - a minimal client that sends a request line and parses the header and payload independently of the host's framing code.
- Characterisation tests, each named for what should hold:
  - an invocation that is not a published subcommand prints the usage line and exits 1;
  - `serve` with an unknown argument, or a non-numeric `--allow-uid`, exits 1;
  - a second `serve` on the same socket path fails while the first holds the lock;
  - a stale socket owned by the user is replaced;
  - a regular file at the socket path is refused and left untouched;
  - SIGTERM and SIGINT each remove the socket and exit 0;
  - `types` lists offered supported images in preference order;
  - `read` returns the exact bytes, NULs and newlines included;
  - `read` of a type that is not offered gets `ok:false`;
  - a payload whose magic does not match gets `ok:false`;
  - a request over 4096 bytes gets `ok:false`;
  - `sync-text` run directly, against stand-ins, writes plain text to the stand-in `xsel` and skips an image offer.
- Do **not** write tests for `F-1`–`F-4` yet. Each lands in phase 3 with its fix, so that it is seen to fail first.
- Admit `crates/host/tests` in `crates/host/Dockerfile` and `.dockerignore`, then confirm the image build runs the layer. With the Docker CLI unavailable, say so in the journal; do not skip silently.
- Apply `D-2`'s amendment:
  - `CONTRIBUTING.md` §Tests;
  - `.agents/docs/test-architecture.md` §The layers and §Failure interpretation;
  - each document's change record, carrying the replaced wording.
- `README.md` §Tests and limitations waits for phase 4, when the coverage it describes is final.

Exit: every new test passes against the unmodified source. The binary is unchanged, so its size is still 619,264 bytes (`I-1`). A different figure means something other than tests changed.

## Phase 2 — Restructure, with no change in behaviour

Decisions: `D-1`, `D-3`, `D-9`, `D-10`, and `D-2`'s sibling-file rule.

Suggested order within the phase, one commit each if that helps review:

1. Workspace `[lints]` table and the `rust-version` bump. `clippy.toml` holds `allow-unwrap-in-tests = true`, if that is the mechanism that keeps `unwrap` available in tests.
2. Split `main.rs` into the modules `D-1` lists, moving code without changing it. Move each test into its module's `tests.rs`.
3. Create the `sys` module: every `unsafe` block moves behind a safe function with a `SAFETY` comment, and `unsafe_code` is denied outside it. `flock` becomes `File::try_lock`.
4. The `D-3` types:
   - the format enum and its magic check;
   - the request enum and its parser;
   - the query enum, which replaces the argument-array seam, with the unit tests rewritten against it;
   - the command enum and `serve` options.
5. Replace `ChildGuard(Child, bool)` with a guard whose type distinguishes reaped from unreaped. Phase 3 changes the mechanism; this step only changes the representation.
6. `D-10`: put text sync's sequence behind injected tool calls, and add the unit tests it lists.
7. `D-9`: delete `handle-stdio`, and add the binary test showing it is now a usage error.

Exit:

- Every phase-1 binary test passes unchanged, and the unit tests pass with only their seam changed.
- `rg -n unsafe crates/host/src` finds nothing outside the `sys` module, apart from lint attributes.
- Measure the release musl binary (`CONTRIBUTING.md` §15). The expected direction is roughly flat. Record the figure and any delta beyond ±2% (about 12 KiB) with its cause, read off `cargo bloat`, in the journal before moving on.

## Phase 3 — Fix the findings, one decision at a time

Each step follows the same rhythm: write the test that the finding predicts will fail, run it and see it fail for the stated reason, apply the decision, then see it pass. Order matters only where noted.

| Step | Decision | Finding  | Failing test first                                                                                                                       |
| ---- | -------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| 3.1  | `D-5`    | `F-4`    | Unit: a listing with a non-UTF-8 line still serves `image/png`. Same for the sync filter.                                                |
| 3.2  | `D-6`    | `F-5`    | Unit: framing into a counting writer makes a fixed number of writes, and the header parsed independently matches the payload length.    |
| 3.3  | `D-6`    | `F-2`    | Unit, with millisecond bounds: a trickling reader and a stalled writer each fail within their deadline.                                  |
| 3.4  | `D-4`    | `F-1`    | Binary: a 64 MiB image from a stand-in `wl-paste` arrives byte-exact. Unit: timeout and reaping still hold, and a reaped child is never signalled. |
| 3.5  | `D-7`    | `F-6`    | Binary: SIGTERM during an in-flight slow request exits only after the request's tool is reaped; SIGHUP removes the socket. Unit: a per-connection failure leaves the loop running. |
| 3.6  | `D-8`    | `F-3`    | Binary: after SIGKILL to `serve --sync-text`, the stand-in watcher is gone.                                                             |

- 3.4 depends on 3.2 only in that both touch the request path. Land 3.2 first so that the 64 MiB test measures capture, not framing.
- 3.5 builds on the guard from phase 2 and the deadlines from 3.3, which make joining workers bounded.
- 3.6 needs the main-thread spawn site that 3.5 establishes.

After 3.4, re-run the `I-1` throughput loop over the new capture function on the same machine, and record it in the journal. The required property is that a 63 MiB capture finishes well inside its 4 s bound. Expect it to be limited by `head` writing into the pipe, with tens to low hundreds of milliseconds as the order of magnitude to expect. That figure is not a budget.

Exit: every finding `F-1`–`F-6` has a test that failed before its step and passes after. Measure the binary again and record the delta and its causes.

## Phase 4 — Close

- `README.md` §Tests and limitations states what the layers now cover. No record identifiers, per `documentation.md` §5.2.
- Mark the register's decisions as implemented, naming their sites and tests, and the findings as settled.
- The journal records the final measurements: binary size against 619,264 bytes; test count against 13; capture throughput against `I-1`.
- Push the branch.
- **Owner:** the manual end-to-end check `test-architecture.md` requires, because the change touches what the host asks of `wl-paste`:
  - README step 5 on Fedora: `wl-paste --list-types`, then `wl-paste --type image/png | wc -c` in the container, then a paste into Claude Code;
  - one large screenshot, the case `F-1` made slow;
  - one `--sync-text` session: copy text, paste it into an X11 application, then copy an image and confirm it is not overwritten.
- **Owner:** start the review round. Its scope is the landing of phases 1–4 against `D-1`–`D-10`.

## Phase 5 — Remediate round `host-initial-refactoring-r1`

Decisions: `D-11`, which supersedes `D-7`, and `D-12`, which supersedes `D-9`. Rulings: `D-4` §Adjudicated, `D-8` §Adjudicated and `F-14` §Ruling. Findings: `F-10`–`F-15`, `F-17`, `F-18` and `F-20`–`F-24`. `F-16` and `F-19` were settled in the record and owe nothing here. `Q-1` stays out of scope.

**Owner:** an `implementer` session. **After it:** a fresh `consolidator` for a second round, scoped to this phase.

The register entries are what is owed, and this section only orders them. The rhythm is phase 3's. Where a defect can be shown, write the test first, see it fail for the reason the finding gives, then fix it. Where it cannot, as with an absence or a comment, the step says so. Commit each step separately.

| Step | Entry                      | Test first, and what it should show before the fix                                                                                                                                       | Change                                                                                                                                                                                                    |
| ---- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 5.1  | `F-15`                     | In a scratch copy only: remove the `is_socket()` limb in `server::serve`, and `should_refuse_and_keep_a_regular_file_at_the_socket_path` hangs. Nothing is committed for this.           | Every binary test that starts `serve` ends it and fails on every outcome, the outcome where `serve` does not refuse included. Do this first, because 5.4 and 5.8 add tests of this kind.                  |
| 5.2  | `D-11` (`F-21`)            | None possible. The change removes a guard against a failure that cannot occur, and `F-21`'s probe is the evidence.                                                                         | Drop `SA_RESTART`, and drop the `Interrupted` arm on `accept`. Make the doc comment on `sys::wake_on_termination` state only what is true.                                                                 |
| 5.3  | `D-11` (`F-10`)            | Binary: `serve` started through `/bin/sh -c 'trap "" HUP; exec …'` dies on SIGHUP.                                                                                                        | Install a handler only for a signal whose disposition is not ignored when `serve` starts.                                                                                                                  |
| 5.4  | `D-11` (`F-11`)            | Binary: with an idle connection open, after SIGTERM the socket path still exists, the idle peer gets nothing, and the exit takes about one request phase.                                  | On the wake byte, stop the watcher, remove the path and close the listener before joining. The request phase ends at shutdown with `bridge is shutting down`. The lock is released last.                  |
| 5.5  | `F-14`, `F-12`             | Binary: 16 idle connections, then a 17th is refused, then a request is served after the 16 close. It passes against the current code. Show its strength instead: in a scratch copy it fails under `F-12`'s `admit` mutation. | No production change. Correct the coverage text that `F-12` names, where it remains unpinned: the UID check and the 12 s phase value are proved at the function level only.                              |
| 5.6  | `D-4` (`F-13`)             | Unit: `F-13`'s probe shape, under a 200 ms deadline, takes about 3 s.                                                                                                                     | Write stdin without blocking, in the capture's `poll` loop, and remove the writer thread.                                                                                                                 |
| 5.7  | `D-8` (`F-17`)             | None. This is a comment.                                                                                                                                                                 | The doc comment on `clipboard::watch` states what happens to a running sync on a graceful shutdown and on parent death.                                                                                    |
| 5.8  | `D-12` (`F-18`, `F-24`)    | Binary: the malformed-`serve` table asserts each message, and `--allow-uid alice` fails on `invalid digit found in string`.                                                               | Name the rejected UID value. Fold `handle-stdio` into the usage-refusal table, and delete the separate test with its unused setup.                                                                          |
| 5.9  | `F-20`, `F-22`, `F-23`     | None. Each is a precondition, a comment or a bound.                                                                                                                                      | Meet `Deadline`'s precondition on the refusal path, or state at the call the assumption the code relies on. Restate `Deadline`'s reason as the adjudicated premise. Remove `Send + Sync` if, after 5.6, no caller needs it. |

Notes on the sequence:

- **5.2–5.4 change the same function, `sys::wake_on_termination`, and the same loop.** Commit them separately all the same, so that each `D-11` limb can be read in its own diff.
- **5.4: the wake pipe stays readable once written,** because nothing drains it. A request-phase wait can therefore include it, with no other state. The response phase must not include it (`D-11`). `F-22`'s comment is rewritten in this step, because `Deadline` changes here.
- **5.4 changes user-facing behaviour.** `README.md` step 2 and §Tests and limitations say what shutdown now does: the socket goes at once, a request already received is answered, and a peer still sending is told the bridge is stopping. They also say that a SIGHUP inherited as ignored, as under `nohup`, keeps the bridge running when its terminal closes. No record identifiers appear in either (`documentation.md` §5.2).
- **5.6 is the only step that touches `sync-text`'s write to X11.** Re-run `should_leave_the_group_of_a_successful_tool_alone` and the sync binary tests after it.

Exit:

- Every step's failing test failed for its stated reason before its fix, and passes after it. Each limb waived in `F-14` §Ruling is named in the journal as waived, not as tested.
- The gates in [`handoff.md`](../../../.agents/docs/handoff.md) §Verify what you changed pass: `cargo fmt`, clippy with `-D warnings`, `cargo test --workspace` and the musl-target test command. The Python suite also runs, because a new error message reaches the shim.
- The release musl binary is measured against 619,264 bytes (`CONTRIBUTING.md` §15). Any delta beyond ±2% is recorded with its cause.
- The register gains `D-11` §Implemented and `D-12` §Implemented, each naming its sites and tests. Each finding this phase settles has its status line updated, and the journal records the phase.
- The branch is pushed. A new round is the owner's to start.

## Out of scope

- `bridge.py` and `test_bridge.py`. The protocol does not change, so the shim and its suite are untouched. The Python suite still runs whenever a phase touches framing, as `handoff.md` requires.
- The text-sync policy itself: which states sync, whether sensitive content syncs, and the image-versus-text race that only the compositor could close.
- `Q-1`, the watcher's death while serving. It waits for the owner. The structure from `D-7` makes the recommended answer a small change.
