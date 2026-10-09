# Host initial refactoring

Turn `crates/host` from a single script-shaped file into a structured crate that is typed and fast, tested in separate files and at the binary's own surface, and that fixes the defects the analysis found on the way.

**Status:** phases 1–4 were implemented on 2026-10-06. Round `host-initial-refactoring-r1` reviewed them on 2026-10-07, and its routed findings were ruled on 2026-10-08. Phase 5, the remediation of that round, was implemented on 2026-10-09. Round `host-initial-refactoring-r2` reviewed it the same day, and its routed findings were ruled on 2026-10-09. Phase 6, the remediation of round r2, was implemented on 2026-10-09. Next are a third review round, scoped to phase 6, the owner's end-to-end check on Fedora and the Docker build.

## Reading order

| Document                         | What it holds                                                                                                     |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [`analysis.md`](analysis.md)     | How the host works today, the properties a refactoring must keep, each finding in detail, and how every measurement was taken |
| [`decisions.md`](decisions.md)   | For each decision, the alternatives considered and why they lost                                                  |
| [`plan.md`](plan.md)             | Six phases: pin behaviour, restructure, fix test-first, close, then remediate the first and the second review rounds. Each with its exit condition |
| [`journal.md`](journal.md)       | Dated log of the work, and the judgement calls an owner may want to revisit                                       |

The entries themselves are canonical in the register, [`00-index.md`](../../00-index.md). Read one with the `awk` line at the top of that file.

## Entries

| Series         | Entries        | In short                                                                                                                                                       |
| -------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Findings       | `F-1`–`F-34`   | Five tier-A runtime defects: capture capped near 14 MiB/s (`F-1`), unbounded slow peers (`F-2`), an orphaned watcher (`F-3`), non-UTF-8 listings (`F-4`), and server lifecycle (`F-6`). Then header syscalls, structure, dead surface and coverage gaps. `F-10`–`F-24` come from the first review round: the lifecycle of signals and shutdown, a stdin overrun, coverage claims, and the record. `F-25`–`F-34` come from the second round: a refusal the shim loses on its send, signal tests that depend on the runner, unwitnessed shutdown ordering, `accept` retries resting on TCP's premise, and smaller items |
| Decisions      | `D-1`–`D-15`   | Modules with `unsafe` confined; sibling test files and a binary test layer; typed domain values; event-driven capture; byte-level listing; total connection deadlines; event-driven shutdown with joined workers; watcher tied to the bridge; `handle-stdio` deleted; text sync behind a seam. `D-11` corrects the lifecycle and supersedes `D-7`; `D-12` corrects the invocation rule and supersedes `D-9`; `D-13` covers commit identifiers already in the record; `D-14` makes reading a refusal the client's job; `D-15` supersedes `D-11`: `accept` retries nothing, and the signal witnesses hold whatever the runner ignores |
| Questions      | `Q-1`          | What the bridge should do when the watcher dies while it serves                                                                                                |
| Investigations | `I-1`–`I-3`    | Baseline and throughput; OS and protocol probes; cost of `serde` derive                                                                                         |
