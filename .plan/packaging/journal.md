# Journal

A dated log of this work: what was done, what was found, what is waiting. Newest last. Entries are append-only.

## 2026-10-09 — Decisions and plan (architect)

**Session.** Architect role on branch `host/initial-refactoring`. `Effort guard active` was present at start. The request: plan the `lexopt` migration and GitHub Actions that produce installable x86_64 `.deb` and `.rpm` packages. Keep `D-12`'s command-line contract. Include the container shim's files, the runtime dependencies, install-and-remove checks and validation of the Docker builder. Require no development tools and no Python on the host. Defer round r3 until both stages are implemented, then review the complete project. `Q-1` stays out of scope.

**Read.**

- `cli.rs`, `main.rs`, the socket-path code in `server::serve`, and both manifests.
- `crates/host/Dockerfile`, `.dockerignore`, `bridge.py`, `setup-host.sh` and `README.md`.
- The binary layer's `cli.rs` and `support.rs`.
- `D-12`, `D-13`, the register's tail, and the refactoring's plan and journal.
- `CONTRIBUTING.md`, by section: §Priorities, §Language and platform, §4, §8 and §15.
- `documentation.md` §6, §8 and §10, `test-architecture.md` §CI policy, `review-findings.md` §Tier, and `handoff.md`.
- `lexopt` 0.3.2's source, for how it handles `--`, `=` and lossy decoding.

**Found.**

- `F-35`: `claude-clipboard-host serve $'\xff'` panics in `env::args()` and exits 101.
- The default socket sits beside the executable, which cannot work for a binary in `/usr/bin`. That made `D-17` a precondition of packaging.
- `test-architecture.md` §CI policy already names the gate that CI must run, and `D-19` adopts it unchanged.
- [`documentation.md`](../../.agents/docs/documentation.md) §10 rules out pinning actions by commit identifier. `D-19` uses only GitHub's own actions, pinned by release tag.

**Read as.** "No host devtools or Python required" is read two ways, and both are kept:

- the installed package requires neither (`D-18`);
- building and checking the packages needs only Docker (`D-19`).

**Not observed.** No Docker CLI is available here, so the builder, the images and the SELinux behaviour of the two-mount layout are unverified. `D-19` makes the workflow run the builder's first validation. `D-18` keeps the README from claiming the SELinux case until the owner's Fedora check.

**Judgement calls an owner may want to revisit:**

- **`--allow-uid=UID` and `serve --` are accepted** as spellings of the published options (`D-16`). `D-12`'s listed messages and its rows are unchanged.
- **The package layout mounts two directories in the container, and sets `CLAUDE_CLIPBOARD_SOCKET` there** (`D-18`). A `setup-host.sh` install keeps one mount, so the README describes two layouts until the owner retires the script.
- **The default socket moves** for a binary run from `dist/` or `target/` (`D-17`).
- **Workflow artifacts only:** no tagged release (`decisions.md` §The builder and the workflow).

**Waiting:**

- Stage 1, then stage 2, each by an `implementer` session.
- The owner's Fedora check.
- Round r3, over the complete project.
- `Q-1` and `Q-2`.

## 2026-10-09 — Stage 1: `lexopt` (implementer)

**Session.** Implementer role on branch `host/initial-refactoring`. `Effort guard active` was present at start. The request: stage 1 of [`plan.md`](plan.md), following `D-16` and `F-35`, then push and hand off for stage 2. The owner also answered `Q-2` with Apache-2.0, to be carried into both package formats in stage 2.

**Done.** `D-16` §Implemented in the register has the sites, the tests and the measurement. `F-35` is settled there. `CONTRIBUTING.md` §Language and platform names `lexopt`, and its change record carries the replaced sentence.

**Commits.** Three steps, each with the gates passing at its end. The support change, which lets `Scratch::run` take OS strings, is its own commit. The new rows and the parser are one commit, because a commit of failing rows alone would break the gates. Each new row was seen to fail before the parser changed, as `D-16` §Implemented records. The record and `CONTRIBUTING.md` are the third.

**Departures from the plan.**

- **`serve -- extra` did not pass already.** The hand parser named `--`, and the row expects `extra`, the argument at fault after the end of options. The row was seen to fail before the change, and passes after it.
- **`serve --bogus=1` now names `--bogus`.** The hand parser named the whole argument. No `D-12` row covers the form. `--sync-text=yes` keeps the whole argument in its message, as the plan's row expects.

**Gates.** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (38 unit, 21 binary) and the same suites for `x86_64-unknown-linux-musl`, all passing. `bridge.py` was not touched.

**Measured.** The release musl binary grew by 12,288 bytes, 2.0%, which is just beyond the plan's ±2%, so `cargo bloat` attributes it in `D-16` §Implemented. The clean release build time did not move measurably.

**Judgement calls an owner may want to revisit:**

- **`lexopt`'s `Error` `Display` is linked but unreachable**, through `?` on `Parser::next`, about 0.6 KiB. Mapping that error to a message of our own would drop it, at the cost of a message for a state the parser cannot reach. Left as `?`, with a comment saying why it cannot fail.

**Waiting:** stage 2, by an `implementer` session.

**`Q-2`, the same session.** The owner chose Apache-2.0. `LICENSE`, the workspace's `license` field and `README.md` §License are a commit of their own, and `Q-2` §Answer in the register says what stage 2 owes: the same value in the packaging manifest, carried into both formats.

**`NOTICE`, the same session.** The owner gave the copyright line, `Copyright 2026 Vladimir Rindevich`. It is in a root `NOTICE` rather than in `LICENSE`'s appendix, which stays as published. `Q-2` §Answer and the plan's stage-2 note name it as the `.deb` copyright file's `Copyright:` field.

**The rename to DClip, the same session.** The owner renamed the product DClip. `NOTICE` and `README.md`'s title say so. The published names that carry `claude` are an architectural question, because `D-17` and `D-18` fix some of them, so they are registered as `Q-3` for the architect. Stage 2 waits on it, so that the packages are first built under their final names.

**Waiting:** `Q-3`, by an `architect` session; then stage 2, by an `implementer` session.
