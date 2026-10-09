# `lexopt` and installable packages

Replace the hand-written command-line parser with `lexopt`, keeping `D-12`'s contract, then build installable x86_64 `.deb` and `.rpm` packages with GitHub Actions. The packages carry the container shim and declare their runtime dependencies, and they need no development tools and no Python on the host. An install-and-remove check, and the Docker builder that has never yet run, are validated on every push.

**Status:** planned on 2026-10-09. Next is stage 1 of the plan. Round `host-initial-refactoring-r3` waits until both stages are implemented, and then reviews the complete project.

## Reading order

| Document                       | What it holds                                                                                          |
| ------------------------------ | ------------------------------------------------------------------------------------------------------ |
| [`decisions.md`](decisions.md) | For each decision, the alternatives considered and why they lost                                       |
| [`plan.md`](plan.md)           | Two stages: the parser, then the socket default, the builder, the packages and the workflow. Then the owner's check and round r3 |
| [`journal.md`](journal.md)     | Dated log of the work, and the judgement calls an owner may want to revisit                            |

The entries themselves are canonical in the register, [`00-index.md`](../00-index.md). Read one with the `awk` line at the top of that file.

## Entries

| Series    | Entries       | In short                                                                                                                  |
| --------- | ------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Findings  | `F-35`        | A non-UTF-8 argument makes the binary panic                                                                               |
| Decisions | `D-16`–`D-19` | `lexopt` over OS strings with `D-12` kept; the default socket in the user's data directory; the `claude-clipboard` packages; one Docker build that the workflow only invokes |
| Questions | `Q-2`         | The packages' license                                                                                                     |
