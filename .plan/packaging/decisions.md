# Decisions: the alternatives and why they lost

The decisions themselves — statement, argument and required properties — are canonical in [`00-index.md`](../00-index.md) as `D-16`–`D-19`. This document keeps what the register entries do not: the options that were weighed, and why each one lost. An implementer owes the required properties in the register, and nothing here.

### The parser (D-16)

Canonical entry: `D-16` — `lexopt`, with `D-12`'s contract kept.

- **Keep the hand parser and read `env::args_os()`.** This would close `F-35` with no dependency. The owner asked for `lexopt` by name, so it is not taken. It is recorded because it is the cheaper fix, should the measurement in `D-16` §Implemented come out out of proportion.
- **`clap`.** Rejected, as in the alternatives to `D-3`. Its size and compile time are out of proportion to two subcommands and two flags.
- **`pico-args`.** In the same class as `lexopt`. Not what the owner asked for, and not measured.
- **Refuse `--allow-uid=UID` and `serve --` to keep `D-12`'s "nothing else" literal.** Rejected. Both are spellings of options that are already published, not new values (`CONTRIBUTING.md` §4). Refusing them would take code written against the library's grammar, for no behaviour a user could want.
- **Pass `lexopt`'s own error text through for the refusals `D-12` does not list.** Rejected. One program would then speak in two voices, `unknown argument: --bogus` beside `invalid option '-x'` (`CONTRIBUTING.md` §12).

### The default socket (D-17)

Canonical entry: `D-17` — `$HOME/.local/share/claude-clipboard/clipboard.sock`.

- **Keep the default beside the executable, and have package users set `CLAUDE_CLIPBOARD_SOCKET`.** Rejected. The packaged binary would fail by default with a permission error on `/usr/bin/clipboard.lock`, and the variable would become a value that every package user has to write (`CONTRIBUTING.md` §4).
- **`$XDG_RUNTIME_DIR/claude-clipboard`.** Rejected. That directory is removed at logout, so a long-lived container's bind mount would point at a deleted directory after the next login. And if a container started before the bridge, Docker would create the mount source owned by root, and `serve` could not bind.
- **Honour `$XDG_DATA_HOME`.** Rejected. The mount source written in a compose file would then depend on a variable the container definition cannot see. The README's path is `$HOME/.local/share/claude-clipboard`, and `CLAUDE_CLIPBOARD_SOCKET` remains the override.
- **Fall back to the home directory only when the executable's directory is not writable.** Rejected. Two rules where one does, and the socket's location would depend on file permissions the user never looks at.

### The packages (D-18)

Canonical entry: `D-18` — `claude-clipboard`, the binary in `/usr/bin` and the shim in `/usr/share/claude-clipboard`.

- **One mount, with the shim copied into the user's directory.** That would need a per-user step after every install or upgrade, and a copied shim goes stale when the host is upgraded, although the two ends of the protocol ship together. Rejected.
- **The host binary embeds `bridge.py` and writes it beside the socket when `serve` starts.** One mount, and a shim that is always current. Rejected: it gives the server a file-installing responsibility it does not otherwise have, and the owner asked for the shim to be in the package. It is the candidate to reconsider if the owner's Fedora check shows that a container may not execute the shim from `/usr/share`.
- **An SELinux file context for `/usr/share/claude-clipboard`, added in a scriptlet with `semanage`.** Rejected. `semanage` is Python, and the owner ruled out Python on the host. It would also make the package carry policy.
- **Relabel the package's directory with `z`.** Rejected. It changes the labels of files the package database owns, and `restorecon`, or an upgrade, reverts them.
- **`/usr/lib/claude-clipboard` or `/usr/libexec` for the shim.** `/usr/share` is taken, because the shim is architecture-independent and is executed only inside a container. Both alternatives would carry the same SELinux question.
- **Hard dependency on `xsel`.** Rejected. `xsel` serves only `--sync-text`, so it is recommended. Both `dnf` and `apt` install recommended packages by default.
- **A systemd user unit in the package.** Not asked for, and `Q-1`, the watcher's death while serving, would shape such a unit. Out of scope.
- **Retire `setup-host.sh`.** Not asked for. With `D-19`, a Fedora user can build the `.rpm` with Docker alone, so the script's remaining case is an install without root. The owner may retire it later, which would leave one layout in the README.

### The builder and the workflow (D-19)

Canonical entry: `D-19` — one Docker build, and the workflow is a thin caller.

- **nfpm**, against `cargo-deb` with `cargo-generate-rpm`, against native `dpkg-deb` with `rpmbuild`, and against `fpm`.
  - nfpm is taken. It writes both formats from one declarative manifest, and it generates no dependencies and rewrites no shebangs, so the shim ships byte-identical and Python is never declared. It comes as an official image, so neither the host nor the builder needs Go.
  - `cargo-deb` with `cargo-generate-rpm` means two configurations of one layout, two tools to compile or fetch into the builder, and packaging metadata in the crate's manifest.
  - `rpmbuild` runs the distribution's post-install scripts. On Fedora, `brp-mangle-shebangs` rewrites `#!/usr/bin/env python3` to `/usr/bin/python3`, which breaks the shim in containers whose Python is elsewhere. Its dependency generators would add an interpreter requirement besides.
  - `fpm` needs Ruby.
- **Workflow steps that build with `actions-rs` or `dtolnay/rust-toolchain`, and package with a marketplace action.** Rejected. That logic would exist only in the workflow's YAML, so it could not be rerun locally, and the Docker builder would go unvalidated. It also adds third-party actions to the supply chain.
- **Pin actions by commit identifier.** Rejected by [`documentation.md`](../../.agents/docs/documentation.md) §10. Only GitHub's own actions are used instead, each pinned to an exact release tag.
- **Keep the Dockerfile in `crates/host/`.** Rejected. Its context and its products are now the repository's, the shim and the packages included.
- **A floating `rust:1-bookworm`.** Rejected for the gates. A new stable release can bring a lint that fails clippy for a reason unrelated to the change, which is exactly what `test-architecture.md` §CI policy refuses.
- **Check installation on the runner itself, on Ubuntu.** Rejected. Fedora is the host the project targets, and a check inside a container is reproducible locally.
- **Publish a GitHub Release on a tag.** Not asked for. Workflow artifacts are what the owner asked for, and a release would make the values in `CONTRIBUTING.md` §4 permanent (§8). It is left to a later decision.
