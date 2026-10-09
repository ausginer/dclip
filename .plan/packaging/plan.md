# Plan: `lexopt`, then installable packages

The order in which `D-16`–`D-19` land, what each stage must leave true, and how it is checked. The decisions are canonical in [`00-index.md`](../00-index.md), and their required properties are the whole of what an implementer owes. This plan only sequences them.

**Owner of each stage:** an `implementer` session, on branch `host/initial-refactoring`. **After stage 2:** the owner's check on Fedora, then round `host-initial-refactoring-r3` over the complete project. No round runs between the stages.

## Principles of the sequence

1. **The parser first, alone.** Stage 1 changes one module and adds one dependency. Its measurement (`CONTRIBUTING.md` §15) is then a measurement of that change and nothing else, and stage 2 packages a binary whose command line is settled.
2. **Show the defect before fixing it.** As in the refactoring's phases, a test that can fail before its change is written first and seen to fail for the stated reason. Where a step cannot fail first, it says what shows the test's strength instead.
3. **Each step is its own commit, and the gates in [`handoff.md`](../../.agents/docs/handoff.md) §Verify what you changed pass at the end of every step.** The Python suite runs whenever `bridge.py` is touched or packaged.
4. **Stage 2 is validated in CI.** The devcontainer has no Docker CLI. Pushing the work branch to start the workflow is part of stage 2, and the push may be repeated as the stage iterates. Every fix is a new commit: no amend, no force-push (`AGENTS.md`).

## Stage 1 — `lexopt` (`D-16`, `F-35`)

| Step | Entry          | Test first, and what it should show before the change | Change |
| ---- | -------------- | ----------------------------------------------------- | ------ |
| 1.1  | `D-16`, `F-35` | Binary, in `D-12`'s two tables: a non-UTF-8 argument as the subcommand, after `serve`, and as an `--allow-uid` value. Each fails with exit status 101 and a panic. Rows for `sync-text --`, `serve -x`, `serve --sync-text=yes` and `serve -- extra` pass already, and they guard the rewrite. Unit, in a new `cli` sibling test file: `--allow-uid=1001` and `serve --` fail with `unknown argument`. A repeated flag and either option order pass already. | None yet. `Scratch::run` takes `OsStr` arguments, or gains a sibling that does, so that a row can carry bytes that are not UTF-8. |
| 1.2  | `D-16`         | The rows from 1.1. | `cargo add lexopt` in `crates/host`. `main` hands the OS-string arguments to `cli::parse`, and `cli::parse` reads them with `lexopt`. `D-12`'s messages are kept exactly, and the new refusals use the same form. No `lexopt` error text reaches the user. |
| 1.3  | `D-16`         | None. This is the record and a rule. | Measure (below). `CONTRIBUTING.md` §Language and platform names `lexopt` as the third dependency and what it is for. Its change record carries the replaced sentence, which says that `libc` and `serde_json` are the two the host carries. |

Notes:

- **`D-12`'s existing rows are not edited.** If one has to change for the suite to pass, the change has broken `D-12`. Stop, and route it to the architect.
- **`sync-text` takes no argument at all, `--` included.** `lexopt` consumes `--` silently, so selecting the subcommand needs care, or `sync-text --` is accepted. Row 1.1 guards this.
- **The measurement** is taken on the release musl binary, at the head before 1.2 and after 1.3: the size, a clean release build time, and `cargo tree -e normal`. Use `cargo bloat` to attribute any delta beyond ±2%. The last recorded size is phase 6's 611,072 bytes. Re-take it, rather than trusting it across a merge ([`documentation.md`](../../.agents/docs/documentation.md) §10).

Exit:

- Every 1.1 row that failed before passes after. `D-12`'s rows pass unmodified.
- The gates in `handoff.md` pass, the musl-target test included.
- The register gains `D-16` §Implemented, with its sites, its tests and the measurement. `F-35`'s status line says it is settled. The journal records the stage.

## Stage 2 — The socket default, the builder, the packages and the workflow (`D-17`, `D-18`, `D-19`)

| Step | Entry  | Test first, and what it should show before the change | Change |
| ---- | ------ | ----------------------------------------------------- | ------ |
| 2.1  | `D-17` | Binary: `serve` with `HOME` in the scratch directory, no `CLAUDE_CLIPBOARD_SOCKET`, and umask 077. Expects the socket at `$HOME/.local/share/claude-clipboard/clipboard.sock`, the directory at 0755, and exit 0 on SIGTERM. Before the change, no socket appears there, because `serve` binds beside the test binary. Remove the socket and lock it leaves there. | The default path and the directory's creation, in `server::serve`. A missing `HOME` exits 1 with one line. `README.md` says where the host's socket is. |
| 2.2  | `D-19` | None. This step moves the builder. | A root `Dockerfile` with a pinned Rust image and a target for each Rust gate: fmt, clippy, the default test and the musl test. A target exports the binary as the README's command does today. Delete `crates/host/Dockerfile`, narrow `.dockerignore` to what the stages read, and update the README's build command. |
| 2.3  | `D-19` | None. | Targets for the Python suite and the guard's `node --test` suite, each in a pinned image. |
| 2.4  | `D-18` | None. | The nfpm manifest under `packaging/`, and a target that runs a pinned nfpm image over the binary from 2.2 and the tracked `bridge.py`. It exports the `.deb` and the `.rpm` to `dist/`. |
| 2.5  | `D-18`, `D-19` | Each check names the property behind each assertion. Its strength is shown in 2.7. | The install-and-remove check targets: Fedora with the `.rpm` and the end-to-end smoke, and Debian stable with the `.deb`. Their scripts live under `packaging/`, in bash with `set -euo pipefail`. |
| 2.6  | `D-19` | None. | The workflow under `.github/workflows/`. Its jobs are `docker build --target …` calls and nothing else. The artifact upload depends on every gate and both checks. |
| 2.7  | `D-19` | The first pushed run is the first time the builder ever runs. Read every job's log, not only its status. | Push the branch, and fix forward until every job passes on the pushed head. Show the checks' strength: a run with one deliberate mutation, made by a commit and reverted by the next, must fail the check that names it. Candidates are dropping `recommends: xsel`, or making `bridge.py` 0644. If no mutation can be run that way, the journal says so. Download the artifacts and confirm their names, versions and architectures. |
| 2.8  | `D-17`–`D-19` | None. This is documentation. | `README.md`: the package install, both container layouts, the autostart line for `/usr/bin`, and CI in §Tests. `test-architecture.md` §CI policy, with a change-record entry. `documentation.md` §8, and `AGENTS.md` §Where things are. The SELinux sentence stays unclaimed (`D-18`). |

Notes:

- **2.1 changes user-facing behaviour** only for a binary run from outside `$HOME/.local/share/claude-clipboard`. `setup-host.sh` is unchanged.
- **A test that fails only in the builder is a finding, not a flake.** Two examples: the suite running as root in the image, and a coreutils older than `test-architecture.md` §The layers requires. Stop and report it. Do not skip the test, weaken it, or `#[ignore]` it to make the run pass.
- **Exact versions** are the implementer's to choose at the time of the step, each a current release. That covers the images, GitHub's actions and the runner label. Name them in `D-19` §Implemented. No commit identifier appears in any tracked file. An image digest, where one is used, is written `sha256:<hex>` (`documentation.md` §10).
- **The guard's suite needs a Node that runs TypeScript directly** (`AGENTS.md` §Evidence). Choose the image accordingly. If the suite needs something the image cannot provide, such as a Claude Code installation, stop and report it rather than dropping the gate.
- **`Q-2` is answered: Apache-2.0** (`Q-2` §Answer). The nfpm manifest in 2.4 sets the same value as the workspace manifest, so the `.rpm`'s `License` tag is `Apache-2.0`, and the `.deb` carries the license in `/usr/share/doc/claude-clipboard/copyright`, with `NOTICE`'s line as its `Copyright:` field. The checks in 2.5 assert both, beside `D-18`'s contents.

Exit:

- Every job of the workflow passes on the pushed head of the branch, and the artifacts install as the checks describe.
- The gates in `handoff.md` pass locally for everything the devcontainer can run, and `bash -n` passes for every new script.
- The register gains `D-17` §Implemented, `D-18` §Implemented and `D-19` §Implemented. The last names the run number, the pinned versions, and the mutation run from 2.7 or the reason there was none. The journal records the stage.
- The branch is pushed.

## After stage 2

1. **The owner's check on Fedora,** with SELinux enforcing:
   - install the `.rpm` from the run's artifacts with `dnf`;
   - start `serve --sync-text` from `/usr/bin`;
   - start a container with `D-18`'s two mounts and its environment;
   - run README step 5, and paste into Claude Code.
   
   The result is recorded as an investigation. A denial goes to the architect with its AVC record (`D-18`). The same check also stands in for the end-to-end check still owed since phase 4.
2. **Round `host-initial-refactoring-r3`,** run by a fresh `consolidator` once the check is recorded. Its scope is the complete project at the pushed head, not a phase:
   - the host crate, `bridge.py` and its tests, and `setup-host.sh`;
   - the Dockerfile, `packaging/` and the workflow;
   - `README.md`, `CONTRIBUTING.md` and `.agents/docs/`;
   - the record's active decisions, `D-1`–`D-19` less those superseded (`D-7`, `D-9` and `D-11`).

## Out of scope

- `Q-1`, the watcher's death while serving.
- A systemd user unit, a GitHub Release on a tag, architectures other than x86_64, and an RPM or APT repository.
- Retiring `setup-host.sh` ([`decisions.md`](decisions.md) §The packages).
