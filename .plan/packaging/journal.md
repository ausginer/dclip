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

## 2026-10-09 — `Q-3`: the names (architect)

**Session.** Architect role on branch `host/initial-refactoring`. `Effort guard active` was present at start. The request: settle `Q-3` with DClip as the product name, and plan a consistent rename of the unreleased published names. Update the affected decisions and the stage-2 handoff, and tell product naming apart from client-specific documentation. No compatibility aliases. `Q-1` stays out of scope.

**Read.**

- `Q-2`, `Q-3`, `D-2`, `D-8`, `D-9`, `D-12`, `D-13`, `D-15`–`D-19`, and this folder's plan, alternatives and journal.
- `CONTRIBUTING.md` §What is being written, §4 and §8.
- `documentation.md` §1, §5.2, §6 and §8, and `handoff.md`.
- Every tracked mention of `claude`, `CLAUDE` and `host-clipboard`.
- `README.md`, `setup-host.sh`, both manifests, and the head of `bridge.py`.
- How `clipboard::watch` names the binary: through `current_exe`, so the watcher does not depend on the name.

**Decided.**

- `D-20` answers `Q-3`. The token is `dclip`, environment variables start with `DCLIP_`, and the binary is `dclip`. The mount points become `/opt/dclip` and `/run/dclip`. There are no aliases.
- `D-21`–`D-23` supersede `D-17`–`D-19` by restating them under those names, because a changed path is a substantive amendment (`documentation.md` §6). `D-22` also carries `Q-2` §Answer's license properties in place of `D-18`'s condition, and `D-23`'s install checks assert them.
- The plan gains step 2.0, the rename: one commit, host and shim together, before anything is built.
- `D-20` tells apart three kinds of `claude`:
  - the product's names, which are renamed;
  - Claude Code as the client, named where its behaviour is the subject;
  - the agent harness, which is not touched.

  `CONTRIBUTING.md` §4 and `documentation.md` §5.2 gain the two rules in step 2.0, with change records.

**Judgement calls an owner may want to revisit:**

- **The binary is `dclip`, not `dclip-host`** ([`decisions.md`](decisions.md) §The names).
- **The mount points are renamed** although they never carried `claude`. This one is narrow, and keeping `/opt/host-clipboard` and `/run/host-clipboard` would change nothing else.
- **The README's title becomes client-neutral.** The owner wrote it with "Claude CLI" in it. `D-20` moves Claude Code into the opening, as the client DClip was written for.
- **The repository and the Cargo package `devcontainer-clipboard-host` keep their names.**

**Noticed, not decided.** `Q-2` §Answer owes a `License` tag in the `.rpm` and a copyright file in the `.deb`, and `D-22` carries exactly that. Neither package is required to ship `LICENSE` or `NOTICE` as files. Apache-2.0 §4(a) and §4(d) ask a redistributor to pass both on, and Fedora's packaging guidelines expect `%license` for the license text. This was outside the request, so `D-22` does not add it. It is the owner's to raise before stage 2's step 2.4.

**Waiting:** stage 2 from step 2.0, by an `implementer` session; then the owner's Fedora check and round r3.

## 2026-10-09 — `LICENSE` and `NOTICE` in the packages (architect)

**Session.** The same architect session. The request: before stage 2, complete the packaging contract. Both formats ship `LICENSE` and `NOTICE` as package-owned files, byte-identical to the repository's, at appropriate paths. Install-and-remove checks are required for them. Update the affected decisions and plan under the record rules. `D-20`'s naming stands, and `Q-1` stays out of scope. This settles what the previous entry left as noticed and not decided.

**Decided.**

- **`D-24` supersedes `D-22`** and adds the files:
  - in the `.rpm`, `%license` files under `/usr/share/licenses/dclip/`;
  - in the `.deb`, plain files in `/usr/share/doc/dclip/`, beside `copyright`;
  - in both, mode 0644 and owned by the package with their directory.

  Adding contents changes what the decision requires, so it supersedes rather than amends (`documentation.md` §6). `D-22` was one entry old and unimplemented, and the rule does not distinguish.
- **`D-25` supersedes `D-23`.** Its checks assert the files' bytes, paths, mode and ownership, `rpm -qL` on Fedora, and their removal. Each check also installs with no documentation exclusion in effect, and asserts that before it installs.
- `D-21`'s citation of the packaged binary's location is re-pointed from `D-22` to `D-24`. That is a corrected citation, made in place.

**Found.** `D-23` as written would have failed on a correct package. Fedora's container image sets `tsflags=nodocs`, which drops the `README.md` that `D-23` asserted at the documentation path. A Debian `-slim` image excludes `/usr/share/doc/*` apart from `copyright`. Both statements come from published sources, not from running the pinned images. The plan's note tells the implementer to read the image's configuration.

**Judgement calls an owner may want to revisit:**

- **The `.deb`'s license files sit in `/usr/share/doc/dclip/`,** so a system that excludes documentation through dpkg drops them and keeps `copyright`. Lintian may report them as `extra-license-file`. No lintian gate runs.
- **No second Fedora install with `nodocs`.** `rpm -qL` stands in for it.

**Waiting:** stage 2 from step 2.0, by an `implementer` session; then the owner's Fedora check and round r3.

## 2026-10-09 — Stage 2: the names, the socket default, the builder, the packages and CI (implementer)

**Session.** Implementer role on branch `host/initial-refactoring`. `Effort guard active` was present at start. The request: stage 2 of [`plan.md`](plan.md), following `D-20`, `D-21`, `D-24` and `D-25`, including CI validation, mutation evidence and artifact checks, then a handoff for the owner's Fedora check. `Q-1` stays out of scope.

**Done.** Each decision's §Implemented in the register has the sites, the tests, the measurements and the evidence. In order:

- 2.0, the rename, in one commit;
- 2.1, the default socket, with its witness failing first;
- 2.2 to 2.6, the builder, the suites' stages, the packages, the checks and the workflow;
- 2.7, four runs to green, the mutation run and the artifacts;
- 2.8, the documentation.

Workflow run 6, on the head that carries every product file, passed every job.

**Commits that do not match their message.** A chained shell command stopped partway through step 2.2. The commit that moves the builder to the root lacks the `binary` stage. The next commit, labelled as adding the shim's and the guard's suites, carries that stage and the suites' `.dockerignore` entries. The following commit adds the suites' stages. No history was rewritten, and the tree at the head is as intended.

**Gates.** Locally: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (38 unit, 23 binary) and the same for `x86_64-unknown-linux-musl`, the Python suite, the guard's suite (228), and `bash -n` for `setup-host.sh` and every script under `packaging/`. In CI, the same gates in the pinned images, and both install checks.

**Judgement calls an owner may want to revisit:**

- **The `.deb`'s `Maintainer` is `Vladimir Rindevich`, with no address.** Debian requires the field, and its policy expects an address. No address was published, because the owner has not chosen one.
- **`packages` builds both install checks before it exports.** `docker build --target packages` therefore takes as long as both checks. In exchange, the files it exports are the ones the checks installed, from the same build.
- **CI's Rust is 1.99.0, the current release. The devcontainer has 1.98.1.** No lint or test differed between the two. CI's binary is 615,296 bytes and the devcontainer's is 627,456, so a size figure names its toolchain.
- **Every image is pinned by digest as well as by tag.** Fedora names a release by one number, so only the digest makes its pin exact. The other images are pinned the same way, for one rule.
- **The runner is `ubuntu-24.04`.** GitHub's runner-images repository also documents `ubuntu-26.04`, but its availability was not checked.
- **The parents of `$HOME/.local/share/dclip` get the user's umask.** Only the directory `serve` creates for the socket is set to 0755 (`D-21` §Implemented).
- **`D-20`'s `git grep` witness matches one line outside `.plan/`.** It is the quote in `CONTRIBUTING.md`'s change record that the same decision requires.

**Not observed here.** SELinux. The checks run without enforcing SELinux and without Wayland. They prove the files, the dependencies, the socket and the shim's path to the host. They do not prove a paste.

**Waiting:**

- the owner's check on Fedora, with SELinux enforcing, as [`plan.md`](plan.md) §After stage 2 describes, using run 6's `dclip-packages` artifact;
- then round `host-initial-refactoring-r3`, by a fresh `consolidator`.
