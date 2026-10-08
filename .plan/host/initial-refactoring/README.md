# Host initial refactoring

Turn `crates/host` from a single script-shaped file into a structured crate that is typed and fast, tested in separate files and at the binary's own surface, and that fixes the defects the analysis found on the way.

**Status:** phases 1–4 implemented on 2026-10-06. Round `host-initial-refactoring-r1` reviewed them on 2026-10-07, and its routed findings were ruled on 2026-10-08. Next: phase 5 of the plan, the remediation, for an implementer. After that come a second review round and the owner's end-to-end check on Fedora.

## Reading order

| Document                         | What it holds                                                                                                     |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [`analysis.md`](analysis.md)     | How the host works today, the properties a refactoring must keep, each finding in detail, and how every measurement was taken |
| [`decisions.md`](decisions.md)   | For each decision, the alternatives considered and why they lost                                                  |
| [`plan.md`](plan.md)             | Five phases: pin behaviour, restructure, fix test-first, close, then remediate the first review round. Each with its exit condition |
| [`journal.md`](journal.md)       | Dated log of the work, and the judgement calls an owner may want to revisit                                       |

The entries themselves are canonical in the register, [`00-index.md`](../../00-index.md). Read one with the `awk` line at the top of that file.

## Entries

| Series         | Entries        | In short                                                                                                                                                       |
| -------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Findings       | `F-1`–`F-24`   | Five tier-A runtime defects: capture capped near 14 MiB/s (`F-1`), unbounded slow peers (`F-2`), an orphaned watcher (`F-3`), non-UTF-8 listings (`F-4`), and server lifecycle (`F-6`). Then header syscalls, structure, dead surface and coverage gaps. `F-10`–`F-24` come from the first review round: the lifecycle of signals and shutdown, a stdin overrun, coverage claims, and the record |
| Decisions      | `D-1`–`D-13`   | Modules with `unsafe` confined; sibling test files and a binary test layer; typed domain values; event-driven capture; byte-level listing; total connection deadlines; event-driven shutdown with joined workers; watcher tied to the bridge; `handle-stdio` deleted; text sync behind a seam. `D-11` corrects the lifecycle and supersedes `D-7`; `D-12` corrects the invocation rule and supersedes `D-9`; `D-13` covers commit identifiers already in the record |
| Questions      | `Q-1`          | What the bridge should do when the watcher dies while it serves                                                                                                |
| Investigations | `I-1`–`I-3`    | Baseline and throughput; OS and protocol probes; cost of `serde` derive                                                                                         |
