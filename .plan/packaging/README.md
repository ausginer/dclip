# `lexopt` and installable packages

Replace the hand-written command-line parser with `lexopt`, keeping `D-12`'s contract, then build installable x86_64 `.deb` and `.rpm` packages with GitHub Actions. The packages carry the container shim and declare their runtime dependencies, and they need no development tools and no Python on the host. An install-and-remove check, and the Docker builder that has never yet run, are validated on every push.

**Status:** planned on 2026-10-09. Stage 1 is implemented. `Q-3` is answered by `D-20`, and stage 2, which opens with the rename to DClip's names, is next. Round `host-initial-refactoring-r3` waits until both stages are implemented, and then reviews the complete project.

## Reading order

| Document                       | What it holds                                                                                          |
| ------------------------------ | ------------------------------------------------------------------------------------------------------ |
| [`decisions.md`](decisions.md) | For each decision, the alternatives considered and why they lost                                       |
| [`plan.md`](plan.md)           | Two stages: the parser, then the names, the socket default, the builder, the packages and the workflow. Then the owner's check and round r3 |
| [`journal.md`](journal.md)     | Dated log of the work, and the judgement calls an owner may want to revisit                            |

The entries themselves are canonical in the register, [`00-index.md`](../00-index.md). Read one with the `awk` line at the top of that file.

## Entries

| Series    | Entries       | In short                                                                                                                  |
| --------- | ------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Findings  | `F-35`        | A non-UTF-8 argument makes the binary panic                                                                               |
| Decisions | `D-16`–`D-25` | `lexopt` over OS strings with `D-12` kept. The names `D-20` sets. Under them, in force: the default socket in the user's data directory (`D-21`), the `dclip` packages with `LICENSE` and `NOTICE` (`D-24`), and one Docker build that the workflow only invokes (`D-25`). `D-17`–`D-19` and `D-22`–`D-23` are superseded |
| Questions | `Q-2`, `Q-3`  | The packages' license: Apache-2.0, answered by the owner. Which published names follow the rename to DClip: all of them, answered by `D-20` |
