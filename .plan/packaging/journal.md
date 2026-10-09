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
