# Plan: `lexopt`, then installable packages

The order in which `D-16`, `D-20`, `D-21`, `D-24` and `D-25` land, what each stage must leave true, and how it is checked. The decisions are canonical in [`00-index.md`](../00-index.md), and their required properties are the whole of what an implementer owes. This plan only sequences them.

**Owner of each stage:** an `implementer` session, on branch `host/initial-refactoring`. **After stage 2:** the owner's check on Fedora, then round `host-initial-refactoring-r3` over the complete project. No round runs between the stages.

## Principles of the sequence

1. **The parser first, alone.** Stage 1 changes one module and adds one dependency. Its measurement (`CONTRIBUTING.md` §15) is then a measurement of that change and nothing else, and stage 2 packages a binary whose command line is settled.
2. **The names before anything is built under them.** Stage 2 opens with the rename (`D-20`), alone, so that every later step, test and package is written once, under DClip's names.
3. **Show the defect before fixing it.** As in the refactoring's phases, a test that can fail before its change is written first and seen to fail for the stated reason. Where a step cannot fail first, it says what shows the test's strength instead.
4. **Each step is its own commit, and the gates in [`handoff.md`](../../.agents/docs/handoff.md) §Verify what you changed pass at the end of every step.** The Python suite runs whenever `bridge.py` is touched or packaged.
5. **Stage 2 is validated in CI.** The devcontainer has no Docker CLI. Pushing the work branch to start the workflow is part of stage 2, and the push may be repeated as the stage iterates. Every fix is a new commit: no amend, no force-push (`AGENTS.md`).

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

## Stage 2 — The names, the socket default, the builder, the packages and the workflow (`D-20`, `D-21`, `D-24`, `D-25`)

`D-17` is superseded by `D-21`, which restates it under `D-20`'s names. `D-18` and `D-19` are superseded twice: by `D-22` and `D-23` for the names, and then by `D-24` and `D-25`, which add `LICENSE` and `NOTICE` to both packages and to their checks. Build to `D-21`, `D-24` and `D-25`.

| Step | Entry  | Test first, and what it should show before the change | Change |
| ---- | ------ | ----------------------------------------------------- | ------ |
| 2.0  | `D-20` | None can fail first: this is a rename. The existing suites are the witness, with only their name literals changed, and `D-12`'s and `D-16`'s rows untouched. After the change, `D-20`'s `git grep` matches only under `.plan/`. | One commit, host and shim together: the `[[bin]]` name, `cli::USAGE`, the program-name prefix in `main`, `server::serve`'s variable, the binary tests' literals and `CARGO_BIN_EXE_dclip`, `bridge.py`'s two variables and `test_bridge.py`, `setup-host.sh`, and the export path in `crates/host/Dockerfile`. In the same commit, the current-state documents `D-20` lists, including the two rule amendments with their change records, and `README.md`'s split between DClip and Claude Code. No alias and no fallback. |
| 2.1  | `D-21` | Binary: `serve` with `HOME` in the scratch directory, no `DCLIP_SOCKET`, and umask 077. Expects the socket at `$HOME/.local/share/dclip/clipboard.sock`, the directory at 0755, and exit 0 on SIGTERM. Before the change, no socket appears there, because `serve` binds beside the test binary. Remove the socket and lock it leaves there. | The default path and the directory's creation, in `server::serve`. A missing `HOME` exits 1 with one line. `README.md` says where the host's socket is. |
| 2.2  | `D-25` | None. This step moves the builder. | A root `Dockerfile` with a pinned Rust image and a target for each Rust gate: fmt, clippy, the default test and the musl test. A target exports the binary as the README's command does today. Delete `crates/host/Dockerfile`, narrow `.dockerignore` to what the stages read, and update the README's build command. |
| 2.3  | `D-25` | None. | Targets for the Python suite and the guard's `node --test` suite, each in a pinned image. |
| 2.4  | `D-24` | None. | The nfpm manifest under `packaging/`, with `D-24`'s license metadata and its per-format license files: `LICENSE` and `NOTICE` as license-type files under `/usr/share/licenses/dclip/` for the `.rpm`, and as plain files in `/usr/share/doc/dclip/` for the `.deb`, each entry limited to its packager. A target runs a pinned nfpm image over the binary from 2.2, the tracked `bridge.py`, `LICENSE` and `NOTICE`. It exports the `.deb` and the `.rpm` to `dist/`. `.dockerignore` admits `LICENSE` and `NOTICE`. |
| 2.5  | `D-24`, `D-25` | Each check names the property behind each assertion. Its strength is shown in 2.7. | The install-and-remove check targets: Fedora with the `.rpm` and the end-to-end smoke, and Debian stable with the `.deb`. Their scripts live under `packaging/`, in bash with `set -euo pipefail`. |
| 2.6  | `D-25` | None. | The workflow under `.github/workflows/`. Its jobs are `docker build --target …` calls and nothing else. The artifact upload depends on every gate and both checks. |
| 2.7  | `D-25` | The first pushed run is the first time the builder ever runs. Read every job's log, not only its status. | Push the branch, and fix forward until every job passes on the pushed head. Show the checks' strength: a run with one deliberate mutation, made by a commit and reverted by the next, must fail the check that names it. Candidates are dropping `recommends: xsel`, making `bridge.py` 0644, or dropping the `.rpm`'s license type from `NOTICE`, which only `rpm -qL` catches. If no mutation can be run that way, the journal says so. Download the artifacts and confirm their names, versions and architectures. |
| 2.8  | `D-21`, `D-24`, `D-25` | None. This is documentation. | `README.md`: the package install, both container layouts, the autostart line for `/usr/bin/dclip`, and CI in §Tests, under `D-20`'s names and its split between DClip and Claude Code. `test-architecture.md` §CI policy, with a change-record entry. `documentation.md` §8, and `AGENTS.md` §Where things are. The SELinux sentence stays unclaimed (`D-24`). |

Notes:

- **2.0 renames every published value**, and that is all it changes. If a test needs more than a name literal changed to pass, or a `D-12` or `D-16` row has to change, the rename has changed behaviour. Stop, and route it to the architect. The record's historical text keeps the old names (`D-20`).
- **2.0 renames nothing in the agent harness** — `.claude/`, `.claude-plugin/`, `CLAUDE.md`, `.scripts/`, the `CLAUDE_*` variables in `.devcontainer/` — and keeps `README.md`'s Claude Code sections where Claude Code's behaviour is the subject (`D-20`). A repository-wide replacement of `claude` is wrong for both.
- **2.1 changes user-facing behaviour** only for a binary run from outside `$HOME/.local/share/dclip`. `setup-host.sh` is unchanged by 2.1.
- **A test that fails only in the builder is a finding, not a flake.** Two examples: the suite running as root in the image, and a coreutils older than `test-architecture.md` §The layers requires. Stop and report it. Do not skip the test, weaken it, or `#[ignore]` it to make the run pass.
- **Exact versions** are the implementer's to choose at the time of the step, each a current release. That covers the images, GitHub's actions and the runner label. Name them in `D-25` §Implemented. No commit identifier appears in any tracked file. An image digest, where one is used, is written `sha256:<hex>` (`documentation.md` §10).
- **The guard's suite needs a Node that runs TypeScript directly** (`AGENTS.md` §Evidence). Choose the image accordingly. If the suite needs something the image cannot provide, such as a Claude Code installation, stop and report it rather than dropping the gate.
- **The license is `D-24`'s:** the metadata from `Q-2` §Answer, and `LICENSE` and `NOTICE` as files the package owns. The `.rpm`'s `License` tag is `Apache-2.0`. The `.deb` carries `/usr/share/doc/dclip/copyright`, with `NOTICE`'s line as its `Copyright:` field. Both carry the two files, byte-identical, at their format's paths. The checks in 2.5 assert all of it, as `D-25` requires.
- **The check images install documentation** (`D-25`). Fedora's container image sets `tsflags=nodocs` in `/etc/dnf/dnf.conf`, and a Debian `-slim` image excludes `/usr/share/doc/*` in `/etc/dpkg/dpkg.cfg.d/`. Read the pinned image's configuration rather than trusting either statement. Override or remove the exclusion for the install, and assert that it is gone before installing. Do not weaken an assertion about a documentation or license file to fit the image.

Exit:

- Every job of the workflow passes on the pushed head of the branch, and the artifacts install as the checks describe.
- The gates in `handoff.md` pass locally for everything the devcontainer can run, and `bash -n` passes for every new script.
- The register gains `D-20` §Implemented, `D-21` §Implemented, `D-24` §Implemented and `D-25` §Implemented. The last names the run number, the pinned versions, and the mutation run from 2.7 or the reason there was none. The journal records the stage.
- The branch is pushed.

## After stage 2

1. **The owner's check on Fedora,** with SELinux enforcing:
   - install the `.rpm` from the run's artifacts with `dnf`;
   - start `/usr/bin/dclip serve --sync-text`;
   - start a container with `D-24`'s two mounts and its environment;
   - run README step 5, and paste into Claude Code.
   
   The result is recorded as an investigation. A denial goes to the architect with its AVC record (`D-24`). The same check also stands in for the end-to-end check still owed since phase 4.
2. **Round `host-initial-refactoring-r3`,** run by a fresh `consolidator` once the check is recorded. Its scope is the complete project at the pushed head, not a phase:
   - the host crate, `bridge.py` and its tests, and `setup-host.sh`;
   - the Dockerfile, `packaging/` and the workflow;
   - `README.md`, `CONTRIBUTING.md` and `.agents/docs/`;
   - the record's active decisions, `D-1`–`D-25` less those superseded (`D-7`, `D-9`, `D-11`, `D-17`, `D-18`, `D-19`, `D-22` and `D-23`).

## Out of scope

- `Q-1`, the watcher's death while serving.
- Renaming the agent harness, the Cargo package `devcontainer-clipboard-host` or the repository (`D-20`).
- A systemd user unit, a GitHub Release on a tag, architectures other than x86_64, and an RPM or APT repository.
- Retiring `setup-host.sh` ([`decisions.md`](decisions.md) §The packages).
