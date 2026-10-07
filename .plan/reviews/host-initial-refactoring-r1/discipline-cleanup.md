# Cleanup pass (code discipline), round host-initial-refactoring-r1

## State read

Round `host-initial-refactoring-r1`, the branch `host/initial-refactoring` as launched; the code of the landing this round reviewed. Read in full: `CONTRIBUTING.md` Parts I and II, `documentation.md` §5, `review-findings.md`, the `D-3` and `D-6` entries including their §Adjudicated clauses.

## Scope

Covered: every file under `crates/host/src/` (cli, clipboard, image, main, process, protocol, server, sync, sys) and each sibling `tests.rs`; `crates/host/tests/binary/` (cli, protocol, server, sync, support); the workspace and crate `Cargo.toml` and `clippy.toml`. Compared against `CONTRIBUTING.md` §Types, §Errors, §Shape, §Tests, §Comments, §1.1, §1.2, §2, §3, §5, §6, §9, §10, and `documentation.md` §5.

Not covered: `bridge.py`, `setup-host.sh` and `README.md` (not touched by the range, apart from README prose, which is not this lens); `.plan/` content beyond `D-3`, `D-6` and the plan; binary size and runtime (no measurement taken); `Q-1` (excluded by the launch). The `D-3` error-precedence change and the `D-6` mechanism change are adjudicated as conforming and are not re-reviewed.

## Findings

### cleanup-1 — The error alias carries `Send + Sync` that no use requires

Tier: C

**Finding.** `crates/host/src/main.rs` declares `pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;`.

**Current behavior / contract.** Every error ends as text in the same thread that produced it. `respond_with`, `write_response`, `capture` and `sync_text` run to completion on one thread. The spawned worker closure in `server::serve` returns `()`. The stdin writer thread in `process::capture` returns `io::Result<()>`, not the alias. `CONTRIBUTING.md` §Errors defines the error as "a boxed message".

**Why it is a problem.** `CONTRIBUTING.md` §Types: "Lifetimes and generics no wider than their callers", and §3 (do not pay for type-level structure nobody uses). The bounds promise that an error may cross a thread boundary, and nothing crosses one.

**Evidence.** In a scratch copy of the tree (outside the repository), I changed the alias to `Box<dyn std::error::Error>`. `cargo check --offline --all-targets` built clean. No call site depends on the bound.

**Required property.** The error type's thread-safety bounds are no wider than what some caller needs. If a future caller moves an error across threads, the bound comes back with that caller. Whether any caller is meant to later is the owner's call.

### cleanup-2 — A binary test pins a deleted invocation by name and repeats a sibling test

Tier: C

**Finding.** `crates/host/tests/binary/cli.rs` `should_refuse_handle_stdio_as_an_unpublished_invocation` runs `handle-stdio`, sets up a stand-in clipboard it never reads, and asserts exit 1 plus the usage prefix. `should_print_usage_and_exit_1_for_an_unpublished_invocation` asserts the same behavior over `[]`, `bogus` and `sync-text extra`.

**Current behavior / contract.** Any argument vector outside `serve ...` and `sync-text` is refused with the usage line. `handle-stdio` is a subcommand that no longer exists, so it is one more unrecognised word.

**Why it is a problem.** `CONTRIBUTING.md` §Tests: each test covers one specific piece of logic, and "a table of inputs that all exercise one behaviour" is one case. This is a second case for the same behaviour. The test also names a removed shape (§8, §10 of Part II: no legacy shapes kept "just in case"), so the test suite carries a deleted word as if it were a contract. Its `scratch.clipboard(...)` setup does nothing for the assertion.

**Evidence.** Read of `cli.rs`. The two tests differ only by the argument value and the unused setup.

**Required property.** The refusal of unknown invocations is pinned once. A test carries no setup its assertion does not depend on.

## Examined and found justified (no defect)

Stated so a silent area is distinguishable from a clean one.

- `process::Tool` and `State::{Unreaped, Reaped}`: owns a real transition. Once reaped, the PID may be reused, so the type makes "never signal again" unrepresentable (§1.2). Not duplicate state.
- `process::capture`'s `eof` / `reaped` bookkeeping and per-state fd selection: a real algorithm. Each flag stops polling a descriptor that stays readable.
- `process::spawn`: two call sites, and it owns the process-group rule that keeps `xsel`'s daemon alive.
- `server::Deadline`: owns the total-deadline rule (`D-6`). `server::Slot`, `Socket` and `Bridge`: Drop guards whose field order is the cleanup order (§1.2).
- The `Arc<UnixStream>` in the accept loop: a scoped worker cannot borrow a per-iteration local, and the Arc lets the loop still carry a refusal when the spawn fails. Not a defensive clone (§9).
- `protocol::respond_with`'s closure and `sync::sync_text`'s three closures: the test seam named in §2.1. `clipboard::x11_text` and `set_x11_text` own the `xsel` argument lists and the deadlines.
- `image::Format` having separate `Jpeg` and `Jpg` variants: documented in the source and required by `D-3`.
- `Request::parse`'s hand-built parse from `Value`: required by `D-3`. Its three-way `type` match lists distinct meanings (§Option, Result).
- `sys.rs`: every wrapper owns an `unsafe` contract (SAFETY comments present). `Poll` / `Interest` own a real layout and borrow invariant.
- `sys::poll` returning `Interrupted`, which all three callers (`process::capture`, `server::Deadline::run`, the accept loop) swallow with the same three-line match. Considered as a duplicated concept (§6), and not filed. The early return is what lets each caller recompute its remaining time, so the repetition has a reason.
- Comments and doc comments: no `D-*`, `F-*`, phase, review or history narration found in `crates/host/src` or `tests/` (grep for identifiers and for "previously", "used to", "no longer", "now"). Comments state constraints and consequences, as `documentation.md` §5.1 asks.
- Test layout: each production module declares `#[cfg(test)] mod tests;` and no `#[test]` sits in a production file. Test names carry `should`. Table tests (`should_reject_invalid_requests`, `should_sync_only_in_the_data_and_sensitive_states`, `should_frame_any_response_in_one_write...`) each exercise one behaviour.
- Dependencies: only `libc` and `serde_json`, as §Language and platform names. No features, no traits with one implementor, no registries, no `async`.

## Count

A: 0, B: 0, C: 2.
