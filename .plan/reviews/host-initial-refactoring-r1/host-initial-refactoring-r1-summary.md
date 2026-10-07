# Review round `host-initial-refactoring-r1` — consolidation

The round reviewed the host refactoring on branch `host/initial-refactoring`: phases 1–4 of [`plan.md`](../../host/initial-refactoring/plan.md), with the `D-3` and `D-6` §Adjudicated rulings. The register ([`00-index.md`](../../00-index.md)) was the reference throughout. `Q-1` was out of scope for every pass.

## The passes

Four independent passes ran in parallel. Each wrote to its own staging area, outside this directory, and none could see another's work. Each report was published here only after all four had stopped. The der pass had left an abandoned probe running in the background, superseded by the run its report cites, and the consolidator killed that probe before publishing.

| Pass      | Report                                                     | Read the same tree | A | B | C |
| --------- | ---------------------------------------------------------- | ------------------ | - | - | - |
| reviewer  | [`implementation-reviewer.md`](implementation-reviewer.md) | yes                | 0 | 0 | 7 |
| integrity | [`coherence-integrity.md`](coherence-integrity.md)         | yes                | 2 | 1 | 3 |
| cleanup   | [`discipline-cleanup.md`](discipline-cleanup.md)           | yes                | 0 | 0 | 2 |
| der       | [`elimination-der.md`](elimination-der.md)                 | yes                | 0 | 0 | 2 |

All four passes reported the same launch commit, so their reports can be merged. Every report states what it covered and what it did not. Every report also states its null results, so a silent area can be told apart from a clean one.

Two passes, reviewer and integrity, independently re-ran the gates and got the same results:

- `cargo fmt --check` and `clippy -D warnings` are clean.
- `cargo test` passes 33 unit and 17 binary tests, both natively and with the musl target the image build uses.
- The Python suite passes.
- The release musl binary is 619,264 bytes.

**Not reviewed by any pass:**

- the Docker image build, because no Docker CLI was available;
- whether a builder's seccomp profile allows `pidfd_open` (integrity raised this);
- the owner's end-to-end check on Fedora;
- the refusal of a socket owned by another UID, because reaching it needs privilege;
- `Q-1`.

## Consolidator verification

The consolidator checked the following against the code at the launch commit:

- **`sys::wake_on_termination`.** It calls `sigaction` for SIGTERM, SIGINT and SIGHUP with `SA_RESTART`, and passes a null old-action pointer, so the inherited disposition is never read.
- **`server::serve`.** `bridge` is a local of `serve`. It drops only after `thread::scope` has returned, and so only after every worker has been joined.
- **`process::capture`.** It drops `tool` before joining the stdin writer.
- **`server::Deadline`'s doc comment** opens with "A socket timeout bounds a single call".
- **`support::Scratch::run`** calls `Command::output()`.
- **The journal's phase-3 entry** names its span by a range of two commit identifiers.
- **The `Result` alias** is bounded by `Send + Sync`.
- **The register text:**
  - `D-6` spells the error header with its keys in the order `ok`, `size`, `error`;
  - `D-7` carries the "as soon as the signal is delivered" property and the `EINTR` clause;
  - `D-8` says a running sync "runs to completion";
  - `D-9` says "any other invocation prints the usage line";
  - `D-9` §Implemented and plan step 7 name `should_refuse_handle_stdio_as_an_unpublished_invocation` as `D-9`'s witness.

The runtime probes were not re-run by the consolidator: integrity's `nohup` and drain timings, reviewer's capture and sync probes, and der's `EINTR` measurement. Each is accepted on the report's own evidence, and the code each one depends on was confirmed above.

## Merges

- **`reviewer-4` and `integrity-4` are one finding.** Both say that `D-6` writes the error header's keys in an order the host has never sent.
- **`reviewer-7` and `integrity-5` are one finding.** Both concern the commit range in the journal.

No other pair describes the same defect. The near-pairs below were kept separate on purpose:

- **`reviewer-2` and `integrity-3`.** They share a cause: no test drives the accept loop in `server::serve` except on its success path. They violate different properties, though. `reviewer-2` is about the plan's exit criterion for `F-6`'s per-connection limb. `integrity-3` is about coverage claims in `README.md` and `test-architecture.md`. One test of the accept loop may settle both, and the summary records that link without merging them.
- **`integrity-6` and `der-2`.** Both concern `server::Deadline`. `integrity-6` is a path that breaks a precondition. `der-2` is a justification that cites a premise the record has retired.
- **`integrity-1` and `der-1`.** Both concern `sys::wake_on_termination`. `integrity-1` concerns which signals the bridge takes over, and `der-1` concerns the `SA_RESTART` flag. Whoever rules on one should read the other.

## Consolidator edits to pass reports

Two reports used a commit identifier as provenance, which `documentation.md` §10 forbids in a tracked file. The consolidator replaced each with the round's name and changed nothing else:

- the base-state citation in `der-1`'s evidence;
- the "State read" line of the cleanup report.

The reviewer and integrity reports still quote the journal's sentence that contains a commit range. Each quote is the evidence for `F-19` and is not used as a reference, so both were left as written.

## Rejections

None. No finding was falsified by evidence.

## Adjustments to a pass's claim

**`cleanup-2`.** The required property "the refusal of unknown invocations is pinned once" conflicts with two binding sources: `D-9` §Implemented and plan step 7. Both require a binary test showing that `handle-stdio` is now a usage error, and both name this test as `D-9`'s witness. The pass did not cite either. The rest of the finding is supported by its evidence: the test's `scratch.clipboard(…)` setup plays no part in its assertion. The finding is kept in full and routed to the architect, because whether the witness should stay is a question about `D-9`, not a cleanup.

## Tiers

Every tier below is the reporting pass's own. Each was checked against the consequence semantics in `review-findings.md` §Tier and none was changed. Two judgements are recorded here so they can be argued with:

- **`reviewer-1` at Tier C.** The overrun is real, but no correctly installed bridge is known to reach it: the only capture that takes input is `xsel -ib`, and `xsel` reads all of its input before forking. If a production trigger is found, the finding becomes Tier A.
- **`reviewer-5` at Tier C.** The reported fault is a statement in the record and a doc comment. The behaviour itself is unchanged from the initial implementation. The possible consequence, a truncated X11 selection when shutdown interrupts a sync, was reasoned from the code and not reproduced. If it is reproduced, the tier should be reconsidered under the first limb of Tier A.

## Findings

Canonical ids are allocated per prefix. The register's highest ids before this round were `F-9`, `Q-1` and `I-3`. This round mints only `F-` ids, `F-10` to `F-24`. Each is registered in [`00-index.md`](../../00-index.md) in the same commit as this summary, so the mapping below is canonical, not proposed.

| Local                      | Canonical | Tier | Claim                                                                                   | Route                       |
| -------------------------- | --------- | ---- | --------------------------------------------------------------------------------------- | --------------------------- |
| integrity-1                | F-10      | A    | A SIGHUP inherited as ignored is overridden, so a `nohup` bridge now dies with its terminal | architect (`D-7`)        |
| integrity-2                | F-11      | A    | During the shutdown drain the socket stays bound and new connections queue unserved     | architect (`D-7`)           |
| integrity-3                | F-12      | B    | README and `test-architecture.md` claim running-server coverage that mutations survive  | implementer                 |
| reviewer-1                 | F-13      | C    | A failed capture with stdin outlives its deadline when the tool's group holds stdin      | architect (`D-4`)           |
| reviewer-2                 | F-14      | C    | `F-6`'s per-connection limb has no test that drives the accept loop                      | owner or architect          |
| reviewer-3                 | F-15      | C    | Three binary tests start `serve` through `Command::output` and hang instead of failing   | implementer                 |
| reviewer-4, integrity-4    | F-16      | C    | `D-6` gives the error header's key order differently from the bytes the host sends       | architect (`D-6`)           |
| reviewer-5                 | F-17      | C    | A graceful shutdown kills an in-flight sync, although `D-8` says it runs to completion   | architect (`D-8`)           |
| reviewer-6                 | F-18      | C    | Malformed `serve` options print a specific message, not `D-9`'s usage line               | architect (`D-9`, `D-3`)    |
| reviewer-7, integrity-5    | F-19      | C    | The journal cites a commit range as provenance                                           | record owner                |
| integrity-6                | F-20      | C    | `serve` hands `Deadline` a blocking stream when `set_nonblocking` fails                  | implementer                 |
| der-1                      | F-21      | C    | `SA_RESTART` and the `accept` `Interrupted` retry rest on a premise that never held      | architect (`D-7`, `F-6`)    |
| der-2                      | F-22      | C    | `Deadline`'s doc comment restates the premise that `D-6` §Adjudicated withdrew            | implementer                 |
| cleanup-1                  | F-23      | C    | The error alias's `Send + Sync` bounds are wider than any caller needs                   | implementer                 |
| cleanup-2                  | F-24      | C    | The `handle-stdio` test repeats the usage-refusal test and has setup it never uses       | architect (`D-9`)           |

Each finding's full problem report is in the pass report its local id names. The register entry summarises it and states its required property.

## Routing

**Architect.** These are contract questions the consolidator may not settle.

- **`F-10` and `F-11` (`D-7`).** Should an inherited `SIG_IGN` be honoured? And is the drain after a signal, during which the socket is still advertised, what "as soon as the signal is delivered" means, and in what order does it come relative to cleanup?
- **`F-13` (`D-4`).** `D-4` requires both "one deadline bounds the whole capture" and "a reaped child is never signalled". On a failure path with a lingering group member, the two cannot both hold.
- **`F-17` (`D-8`).** Is the "runs to completion" property limited to parent death?
- **`F-18` (`D-9` against `D-3`).** Does "any other invocation" cover malformed options of `serve`?
- **`F-21` (`D-7`, `F-6`).** Should the `EINTR` clause be kept? Whatever the answer, the code changes follow the ruling.
- **`F-16`.** Correct `D-6`'s text.
- **`F-24`.** Should `D-9`'s witness stay?

**Owner or architect.** `F-14`: either a test is added, or someone waives that limb of the plan's exit criterion and the waiver is recorded.

**Record owner.** `F-19`: the journal is append-only, so how to correct it is that owner's call.

**Implementer.** `F-12`, `F-15`, `F-20`, `F-22` and `F-23`. Each can be fixed without deciding anything.

- `F-12` can be met either by adding tests or by correcting the coverage text. Both are allowed by its required property, and choosing one is not a contract decision.
- A test that drives the accept loop for `F-12` may also serve as `F-14`'s test. That is worth seeing before either is done alone.

**Disagreements.** None. No two passes made conflicting claims about the same thing.
