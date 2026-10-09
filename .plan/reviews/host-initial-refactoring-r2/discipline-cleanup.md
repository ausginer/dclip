# Code discipline review, round host-initial-refactoring-r2 (cleanup pass)

## State read

The state of the branch `host/initial-refactoring` after Phase 5 of `plan.md` ("Remediate round `host-initial-refactoring-r1`"), with the register as it stands after `D-11` and `D-12` are recorded as implemented. Sections cited: `CONTRIBUTING.md` Part I (Types, Errors, Tests, Comments) and Part II §1.1, §1.2, §2.1, §9; `documentation.md` §5.

## Scope

Covered: the changes Phase 5 made in `crates/host/src/` (`cli.rs`, `clipboard.rs`, `main.rs`, `process.rs`, `server.rs`, `sys.rs` and their sibling `tests.rs` files) and `crates/host/tests/binary/` (`cli.rs`, `server.rs`, `support.rs`). That includes the concurrency test's retry loop and the signal tests' handling of inherited dispositions. Not covered: code Phase 5 did not touch, `bridge.py`, `Q-1`, lifecycle and failure semantics as such (the architect's, `CONTRIBUTING.md` §13), and whether the behaviours are correct (other lenses).

Considered and found justified, so no finding:

- `Poll::optional` and `Poll::new` as a thin wrapper over it. One array now watches a changing subset in `capture` and in `Deadline`, and it replaced the three-way slicing `match`.
- `Deadline::or_until_shutdown` with the `stop` field. It owns a protocol rule: only a wait ends early, never bytes already sent. It is the seam the unit tests use.
- The stdin write inside `capture`'s `poll` loop, with `unwritten` and the early `feed = None`. It replaced a thread and a join and owns the "judge the tool by how it exits" rule.
- The signal-disposition probe in `wake_on_termination` (`D-11`), and the dropped `SA_RESTART` and `Interrupted` arm (`F-21`).
- `Bridge` losing `_lock`, and `let bridge = bridge;` in the scope. Both carry the ordering the comments state.
- The shorter `Result` alias, and `cli::parse` naming the rejected value (`D-12`).
- `Scratch::run`'s kill-after-`PATIENCE`, the banner-reading thread in `start`, the `Running` guard and `environment`. Each is `F-15`'s requirement, and each has one owner.
- The merged `handle-stdio` row and the exact-message assertion in the `cli.rs` tests. They honour `F-24`.
- The 4 MiB `capture` test with `==` inside `assert!`. It avoids printing the payload on failure.

## Findings

### cleanup-1 — `set_x11_text` takes an owned `Vec<u8>` that nothing in it owns any more

Tier: C

- **Current behavior / contract.** `clipboard::set_x11_text(text: Vec<u8>)` now only lends the buffer: it calls `capture("xsel", &["-ib"], Some(&text), …)`. `capture`'s `input` was narrowed in this phase from `Option<Vec<u8>>` to `Option<&[u8]>`, because the writer thread that needed ownership is gone. The sole caller, `sync::sync_text`, takes its seam as `impl FnOnce(Vec<u8>) -> Result<()>` and passes it `text`, which it has already compared against `x11_text()`.
- **Why it is a problem.** The ownership transfer no longer has a reason (`CONTRIBUTING.md` §9, and §Types, "take the narrowest thing that works"). The phase narrowed `capture` and left the one wrapper above it wide. A reader sees a move and looks for what consumes it.
- **Evidence.** `crates/host/src/clipboard.rs`, `set_x11_text`. `crates/host/src/sync.rs`, the `set_x11_text: impl FnOnce(Vec<u8>)` parameter and its call.
- **Required property.** A function's parameter type states what the function does with the value. The text seam and its implementation borrow, or the code states what the ownership is for. The seam's type is `sync.rs`'s test boundary, so changing it is the owner's call.

### cleanup-2 — The concurrency test's retry loop absorbs every I/O error as "one more attempt", and the behaviour it works around is recorded only in the journal

Tier: C. Routed to the architect, because the behaviour behind it touches failure semantics (`CONTRIBUTING.md` §13).

- **Current behavior / contract.** In `should_refuse_a_connection_over_the_limit_and_keep_serving`, the final loop treats `write_all(..).and_then(read_to_end)` returning any `Err` as "not yet" and retries until `PATIENCE`. The journal records the cause: `serve` closes a refused stream with the request unread, and Linux can reset the connection before the peer reads the refusal. The loop's own comment says "can reset it". The cause is in the journal only. No register entry states that a refusal can reach the container as a reset, although the message for a refused UID is the one carrying a remedy.
- **Why it is a problem.** Two things, both small.
  - The workaround is wider than its cause. It is not restricted to `ConnectionReset`, so a `BrokenPipe` from a worker that died would also be retried. The test still fails when no place frees (`no place came free`), so it stays sound as an instrument, but the retry hides which error occurred.
  - A known runtime behaviour, a refusal replaced by a reset, is absorbed by test machinery and not carried by the record that decides it (§1.1(a): the peer can reach it).
- **Evidence.** `crates/host/tests/binary/server.rs`, the loop at the end of the test. `.plan/host/initial-refactoring/journal.md`, Phase 5 entries "A flake, found and fixed in the test" and "A refusal can be lost to a reset".
- **Required property.** A test workaround names the condition it tolerates and no other. The behaviour it tolerates is either ruled in the register as accepted or recorded as a finding. This pass does not say which.

### cleanup-3 — `serve_ignoring` takes two parameters that its only caller fixes

Tier: C

- **Current behavior / contract.** `Scratch::serve_ignoring(signal: &str, args: &[&str])` has one caller, which passes `"HUP"` and `&[]`. The `signal` value is spliced into a `trap` command line.
- **Why it is a problem.** The helper is wider than its use (`CONTRIBUTING.md` §Types, "no wider than their callers"). Another signal would test a different row of `D-11`'s "all three signals", and the one test covers only SIGHUP. That is a coverage statement, not a defect. The wide signature suggests coverage the suite does not have.
- **Evidence.** `crates/host/tests/binary/support.rs`, `serve_ignoring`. `crates/host/tests/binary/server.rs`, `should_keep_a_signal_ignored_when_serve_inherits_it_ignored`.
- **Required property.** The helper's parameters match what the suite varies, or the suite varies them.

## Result

Three findings, all tier C. No machinery found that Phase 5 added without an owning value, rule or protocol transition beyond the above.
