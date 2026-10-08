# Implementation review: feature proof

Round `host-initial-refactoring-r1`, reviewer pass (feature proof). The pass read the landing the round reviews, on branch `host/initial-refactoring`: phases 1–4 of [`plan.md`](../../host/initial-refactoring/plan.md) against `D-1`–`D-10` and `F-1`–`F-9`, including the `D-3` and `D-6` `§Adjudicated` rulings. It worked without reading any other pass's artifact or staging area.

## Scope

**Covered.**

- **Production source.** Every module under `crates/host/src/`, read in full. Each was checked against the required properties of `D-1`–`D-10`. Parity was checked against the initial implementation's `main.rs`, which the round's base state carries.
- **Tests.** Every unit test file (`*/tests.rs`) and the binary layer under `crates/host/tests/binary/`, read against the plan's per-phase test lists and the tests named in each `§Implemented`. Every test the register names exists, exactly once.
- **Record and documentation.** The register's `§Implemented` and `§Adjudicated` sub-clauses, the journal, the `decisions.md` amendment, and the documentation amendments the plan requires: `CONTRIBUTING.md` §Tests, `test-architecture.md` §The layers and §Failure interpretation, the change records, and `README.md` §Tests and limitations, step 2 and the build table.
- **Gates, re-run in this pass:**
  - `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  - `cargo test --workspace`: 33 unit and 17 binary tests passed.
  - `cargo test --locked --target x86_64-unknown-linux-musl`, the image build's test command run without Docker: 33 and 17 passed.
  - The Python suite: OK.
  - Release musl binary: 619,264 bytes, as the journal records.
  - No `unsafe` outside `sys.rs`, and no `#[test]` in a production file.
  - The branch matches its remote.
- **Probes**, each in a scratchpad copy of the crate or against the debug binary in a scratch directory. None was added to the tree.

**Not covered.**

- **`Q-1`.** Out of scope, as instructed.
- **The Docker image build.** Not run, because there is no Docker CLI. Only its test command was run, as above.
- **The owner's manual Fedora check.**
- **That each phase-3 test failed before its fix.** Accepted from the journal and not replayed commit by commit.
- **`bridge.py`.** Untouched by the landing. Only its suite was run.
- **Machinery and visibility excess, cross-module coherence, and `CONTRIBUTING.md` Part II cost policy.** These belong to other lenses. This pass did not judge them.
- **One limb of `F-9`'s "refusal of a socket path the user does not own".** The limb where a socket at the path is owned by another UID has no test, and none was looked for, because reaching it needs a second account or privilege. The non-socket limb is tested.

**Conforming, stated so that silence is not read as unreviewed.**

- `D-1`, `D-3`, `D-5` and `D-10` hold as stated. For `D-3`, that is read together with its `§Adjudicated` ruling.
- `D-4` holds on the success and timeout paths, and the buffer stays at exactly 64 MiB of capacity for an image at the limit (probe below).
- `D-6`'s deadline and framing properties hold.
- `D-7` holds: the wake pipe, joined workers, per-connection refusals and the cleanup order are all present.
- `D-8` holds on the parent-death path.
- `D-9` holds at subcommand level.
- `D-2` holds apart from `reviewer-3`.

The exceptions follow.

## Findings

Tier counts: A 0 · B 0 · C 7.

### Finding reviewer-1 — Tier C — A failed capture with input outlives its deadline when the tool's group keeps stdin open

- **Finding.** `process::capture` does not return within its deadline in one case: the tool exits non-zero while another member of its process group holds the stdin pipe and is not reading it. The stdin writer is joined after `drop(tool)`. Because the child is `State::Reaped`, that drop signals nothing, so the join waits for the group member to go away.
- **Current behaviour / contract.** `D-4` requires two things that meet here. "One deadline bounds the whole capture — reading, writing stdin and waiting for exit". And "a child that has been reaped is never signalled". The implementation keeps the second and so loses the first. The initial implementation killed the group on every failure, so its capture returned at once. That is a change in observable timing and failure semantics (`CONTRIBUTING.md` §13).
- **Why it is a problem.** The deadline is the bound `D-7`'s bounded shutdown and `D-6`'s slot accounting are argued from. Here it is not a bound. The same rule also leaves any group member of a tool that failed, or that timed out after its leader exited, running past the capture. That is in tension with `D-7`'s "no tool spawned for a connection outlives the process".
- **Consequence today.** The only capture with input is `clipboard::set_x11_text` (`xsel -ib`), in `sync-text`, not in the server. Real `xsel -i` reads all its input before it forks, so no production trigger is known. That is why the tier is C. The tension itself sits inside `D-4`, so it is routed to the architect, not filed as a defect for the implementer.
- **Evidence.** A test was added to a scratchpad copy of `process/tests.rs`:

  ```
  capture("sh", &["-c", "exec 3<&0; sleep 3 <&3 >/dev/null & exit 1"], Some(vec![b'x'; 1 << 20]), Duration::from_millis(200))
  ```

  - Reviewed state: `Err("sh failed; check it on Fedora")` after 3.013 s.
  - The same probe against the initial implementation's `capture`: the same error after 2.45 ms.
- **Required property.** Whatever `D-4` resolves the two properties to, a capture's total duration, stdin join included, must have a bound that holds on its failure paths.

### Finding reviewer-2 — Tier C — The per-connection limb of `F-6` has no test that exercises the accept loop

- **Finding.** Plan step 3.5 requires "Unit: a per-connection failure leaves the loop running". The phase-3 exit also requires that every finding has "a test that failed before its step and passes after". For `F-6`'s third defect, "one connection can stop the server", neither exists.
- **Current behaviour / contract.** `D-7` §Implemented names three unit tests:
  - `should_refuse_a_peer_whose_uid_is_not_allowed`;
  - `should_refuse_a_connection_while_every_place_is_taken`;
  - `should_release_a_place_when_its_connection_is_done`.

  They pin `admit`'s refusal messages and its slot accounting. None of these was the defect, because the initial implementation already refused those cases and carried on. The paths that did end `serve` were:

  - `?` on the socket timeouts;
  - `EINTR` or another non-`WouldBlock` `accept` error;
  - the `thread::spawn` panic.

  Their replacements, the per-connection branches of the loop in `server::serve`, are untested. The journal's phase-3 entry acknowledges this and argues the fix is structural.
- **Why it is a problem.** `F-6` is marked settled, and the plan's exit criterion is the stated evidence for settling. For this limb the evidence is code reading. A regression that reintroduced a `?` in the loop body would pass every test.
- **Evidence.** `crates/host/src/server/tests.rs` calls `admit` directly. Nothing in that file or in `crates/host/tests/binary/server.rs` serves a request after a refused or failed connection.
- **Required property.** Either a test shows that a per-connection failure leaves `serve` accepting, or the record says explicitly that this limb of the plan's exit criterion was waived and by whom. Whether a waiver is acceptable is the owner's or the architect's call.

### Finding reviewer-3 — Tier C — Three binary tests start `serve` with no path that terminates it

- **Finding.** `Scratch::run` runs the binary through `Command::output()`, which waits for exit. Three tests start `serve` through it and expect it to refuse:
  - `should_refuse_a_second_instance_while_the_first_holds_the_lock`;
  - `should_refuse_and_keep_a_regular_file_at_the_socket_path`;
  - `should_exit_1_for_malformed_serve_arguments`.

  If a regression makes `serve` start instead, the test blocks indefinitely. It neither fails nor terminates the server.
- **Current behaviour / contract.** `D-2` requires: "A test that starts `serve` terminates it on every path, including a failed assertion." `Server`'s `Drop` provides that for `Scratch::serve`, but not for `Scratch::run`.
- **Why it is a problem.** The layer's diagnostic value is that a regression becomes a named failure. Here it becomes a hang of `cargo test`, and of the image build, which runs the layer with no timeout. A bound `serve` is left holding the scratch socket for as long as the hang lasts.
- **Evidence.** In a scratchpad copy, the `is_socket()` limb of the socket-path check in `server::serve` was removed. `timeout 20 cargo test --test binary should_refuse_and_keep_a_regular_file_at_the_socket_path` then produced no result until `timeout` killed it at 20 s.
- **Required property.** Each binary test that starts `serve` ends it, and reports a failure, on every outcome, including the outcome in which `serve` does not refuse.

### Finding reviewer-4 — Tier C — `D-6`'s byte form of the error header is not the bytes the host sends, before or after

- **Finding.** `D-6`'s required property spells the error header as `{"ok":false,"size":0,"error":"…"}`. `protocol::write_response` builds it with `json!`, and the crate does not enable `serde_json`'s `preserve_order` feature, so the keys serialise in sorted order.
- **Current behaviour / contract.** The bytes are unchanged from the initial implementation, which used the same macro. So the property's intent, "unchanged", holds, and its literal byte form does not, and never did. `D-6` §Adjudicated states that every required property holds.
- **Why it is a problem.** The register is consulted instead of the code. A test or a second client written from the decision's literal text would assert bytes the host has never sent. The shim parses JSON, so it is unaffected. A decision's text is the architect's to correct, so this is routed, not fixed.
- **Evidence.** The debug binary ran with a stand-in `wl-paste` that exits 3. The responses were:

  ```
  b'{"error":"wl-paste failed; check it on Fedora","ok":false,"size":0}\n'
  b'{"error":"unsupported operation","ok":false,"size":0}\n'
  ```
- **Required property.** The record's statement of the header's bytes matches the bytes the host sends.

### Finding reviewer-5 — Tier C — An in-flight text sync is killed on a graceful shutdown, against `D-8`'s last property as written

- **Finding.** `D-8` says: "A sync process already running when the watcher dies runs to completion within its own tool deadlines." The doc comment on `clipboard::watch` repeats it without condition. On SIGTERM, SIGINT or SIGHUP, however, `Bridge`'s drop runs `Tool`'s `Drop`, which sends SIGKILL to the watcher's whole process group. Each `sync-text` that `wl-paste --watch` started is in that group, so it is killed mid-run. The property holds only on the parent-death path, where `PR_SET_PDEATHSIG` signals the watcher alone.
- **Current behaviour / contract.** The behaviour is unchanged from the initial implementation, and `D-4` names "killing the watcher's group at shutdown" as intended. So either `D-8`'s property is scoped to parent death and its wording is wider than meant, or the graceful path breaks it. That is a contract question for the architect.
- **Why it is a problem.** The record and the doc comment state an outcome that the most common shutdown path does not have. If the sync is killed while its writer thread is feeding `xsel -ib`, which survives in its own group, `xsel` reads a short stdin. That may leave a truncated selection. This consequence was reasoned from the code, not reproduced.
- **Evidence.** The debug binary ran `serve --sync-text` with a stand-in watcher. The watcher started a child in its own process group, which recorded its PID, slept 5 s and then wrote `sync.done`. The child had PGID 97930, the watcher's group. After SIGTERM to `serve`, which exited 0, the child was gone within 0.2 s, and `sync.done` was never written.
- **Required property.** The record and `clipboard::watch`'s comment state truthfully what happens to an in-flight sync on each way the bridge ends.

### Finding reviewer-6 — Tier C — Malformed `serve` options do not print the usage line that `D-9` requires for "any other invocation"

- **Finding.** `D-9` requires: "The binary accepts `serve [--sync-text] [--allow-uid UID]…` and `sync-text`, and nothing else. Any other invocation prints the usage line and exits with status 1." Malformed `serve` options exit 1 with a specific message instead:

  | Invocation                | Message                    |
  | ------------------------- | -------------------------- |
  | `serve --bogus`           | `unknown argument: --bogus` |
  | `serve --allow-uid alice` | `invalid digit found in string` |
  | `serve --allow-uid`       | `missing UID`              |
- **Current behaviour / contract.** The messages are unchanged from the initial implementation. That conforms to `D-3`'s "user-facing error text is unchanged". The phase-1 test `should_exit_1_for_malformed_serve_arguments` asserts only the exit status. `should_print_usage_and_exit_1_for_an_unpublished_invocation` covers only invocations that choose no published subcommand.
- **Why it is a problem.** Two properties disagree in the letter: `D-9`'s "any other invocation" and `D-3`'s unchanged text. The implementation follows `D-3`. Whether `D-9` means subcommand selection only is a contract reading for the architect. The user-visible effect is a more specific message than the usage line, so the tier is C.
- **Evidence.** The debug binary was run with each of the three invocations above. Each exited 1 with the message shown. `bogus` exited 1 with the usage line.
- **Required property.** The record states one rule for what a malformed `serve` invocation prints, and the binary follows it.

### Finding reviewer-7 — Tier C — The journal cites a commit range as provenance

- **Finding.** The journal's phase-3 entry ends by giving the span in which a defect existed as a range of two commit identifiers.

  > 2026-10-08 — This quote reproduced the identifiers. It was replaced in place under `D-13`, and they are not carried.
- **Current behaviour / contract.** `.agents/docs/documentation.md` §10: "no tracked file uses [a commit identifier] as a reference … not … as evidence, as a status or as provenance". The architect's 2026-10-07 journal entry noticed the same thing and left it for this round.
- **Why it is a problem.** The branch can land squashed, and the identifiers would then resolve to nothing. The sentence carries nothing a reader can check.
- **Evidence.** `.plan/host/initial-refactoring/journal.md`, the paragraph "A phase-2 defect, found and fixed in 3.4."
- **Required property.** The record names the affected states by §10's vocabulary, not by commit identifiers. The journal is append-only, so how the correction is recorded is for whoever owns the record.
