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
