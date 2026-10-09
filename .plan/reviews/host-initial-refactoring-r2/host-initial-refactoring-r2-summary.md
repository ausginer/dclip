# Review round `host-initial-refactoring-r2` — consolidation

The round reviewed Phase 5 of [`plan.md`](../../host/initial-refactoring/plan.md), "Remediate round `host-initial-refactoring-r1`", on branch `host/initial-refactoring`. It was read against the register ([`00-index.md`](../../00-index.md)) and its rulings: `D-11`, which supersedes `D-7`; `D-12`, which supersedes `D-9`; the §Adjudicated and §Remediated clauses of `D-4` and `D-8`; and the rulings on `F-10` to `F-24`. Every pass was also asked to examine three things the journal's Phase 5 entries report:

- a refusal lost to a connection reset;
- the concurrency test's retry, which works around that;
- the termination-signal tests' assumption that the test runner has not inherited ignored signals.

`Q-1` was out of scope for every pass.

## The passes

Four independent passes ran in parallel. Each wrote to its own staging area outside this directory, and none could see another's work. The reports were published here only after all four had stopped. Each pass reported that no probe of its own was still running, and the consolidator confirmed that none was.

| Pass      | Report                                                     | Read the same tree | A | B | C |
| --------- | ---------------------------------------------------------- | ------------------ | - | - | - |
| reviewer  | [`implementation-reviewer.md`](implementation-reviewer.md) | yes                | 1 | 1 | 1 |
| integrity | [`coherence-integrity.md`](coherence-integrity.md)         | yes                | 1 | 2 | 2 |
| cleanup   | [`discipline-cleanup.md`](discipline-cleanup.md)           | yes                | 0 | 0 | 3 |
| der       | [`elimination-der.md`](elimination-der.md)                 | yes                | 0 | 1 | 3 |

All four passes reported the same launch commit, so their reports can be merged. Every report states what it covered and what it left out, and states its null results explicitly.

Reviewer and integrity each re-ran the gates, independently, with the same results:

- `cargo fmt --check` and `clippy -D warnings` are clean.
- `cargo test` passes 37 unit and 19 binary tests, both natively and on the musl target.
- The Python suite passes.
- The release musl binary is 611,072 bytes.

Reviewer also replayed this state's binary tests against the source as it was before Phase 5. Exactly the three tests added in steps 5.3, 5.4 and 5.8 failed there.

**Not reviewed by any pass:**

- the Docker image build, because no Docker CLI was available, so whether the build's `RUN` environment hands the tests default signal dispositions is also unknown;
- the owner's end-to-end check on Fedora;
- a refused UID through the running binary, because that needs a second account;
- `Q-1`.

## Consolidator verification

The consolidator checked these claims against the code at the launch commit:

- **`bridge.py` `request_host`** connects, calls `sendall` on the request, and only then reads the header line.
- **The accept loop in `server::serve`** retries `WouldBlock` and `ConnectionAborted`, with the comment "The readiness was taken by a connection that vanished".
- **The binding `let bridge = bridge;`** moves `bridge` into the loop's closure. `drop(lock)` comes after the scope.
- **The concurrency test's comment** says a refused connection "can reset it before the refusal is read".
- **`Scratch::serve_ignoring(signal, args)`** has one caller, which passes `"HUP"` and `&[]`.
- **`clipboard::set_x11_text`** takes `Vec<u8>`.
- **`documentation.md` §5.1** still uses the stdin-writer join as its example of a working comment.
- **The folder `README.md`'s Status line** says "Next: phase 5".
- **`README.md` step 2** says "closing it, Ctrl+C or SIGTERM stops the bridge".

The consolidator did not re-run the runtime probes behind these findings: signal dispositions, refusal delivery, the mutations, or the `accept` behaviour on `AF_UNIX`. Each is accepted on its report's evidence, and the code each one depends on was confirmed above.

## A factual disagreement, settled by the evidence

Three passes examined the journal's "a refusal can be lost to a reset".

**Where all three agree:**

- The refusal's bytes are always delivered ahead of any reset.
- A peer's read returns `ECONNRESET` only after those bytes, at the point where it would have seen end of stream.
- A send that comes after the bridge has closed fails with `EPIPE`. der saw this 50 of 50 times, integrity 100 of 100 with a 2 ms pause, and reviewer 20 of 20 with a 50 ms pause.

The journal and the test comment, which place the loss on the read, are therefore wrong about the mechanism. `reviewer-3` and `der-3` both say so.

**Where they disagree:** whether the container ever loses a refusal.

- `der-3` says the shim "never" exhibits it. The shim reads by the framing, and 30 of 30 runs through `bridge.py` printed the refusal.
- `integrity-1` and `reviewer-1` say the shim loses it on its `sendall`. The bridge can close before the shim sends, and the shim then raises `BrokenPipeError` without reading the refusal already in its receive queue.
  - Integrity ran the real `bridge.py`: 1 of 118 refusals were lost on an idle machine, and 22 of 39 under CPU load.
  - Reviewer reproduced the shim's sequence: 91 of 200 were lost on an unloaded host.

**The evidence settles it.** The shim sends before it reads, which the consolidator confirmed, and all three passes agree that a send after the close fails. Whether the shim loses the refusal therefore depends only on timing. A 30-run sample at an idle loss rate of about 1 in 118 would be expected to show no loss, so der's null result does not count against the positive results from the real shim. `der-3`'s claim about the mechanism is upheld and merged into `F-28`. Its limb saying the shim cannot exhibit a lost refusal is rejected (see Rejections).

## Merges

- **`F-25`** merges `reviewer-1` and `integrity-1`, plus `cleanup-2`'s second limb. That limb asked for the behaviour to be carried by the register rather than only by the journal, and registering `F-25` meets it.
- **`F-26`** merges `der-1`, `reviewer-2` and `integrity-3`. All three are one defect: the termination-signal binary test depends on the dispositions the test runner inherited. The three passes agree on Tier B and have matching reproductions.
- **`F-28`** merges `reviewer-3`, `der-3`'s first limb and `cleanup-2`'s first limb. All three concern the same piece of test machinery:
  - its comment gives the wrong mechanism (`reviewer-3`, `der-3`);
  - it tolerates any I/O error rather than the failures that actually occur (`cleanup-2`, `der-3`).

**Kept separate but linked:**

- **`F-30` and `F-31`.** `F-30` is about coverage: only SIGHUP's inherited ignore is tested, and `README.md` step 2 names SIGHUP alone. `F-31` is that `serve_ignoring` takes parameters wider than its single use. If the suite comes to vary the signal for `F-30`, `F-31` goes away. They violate different properties, so they stay two findings.
- **`F-26` and `F-30`.** Both concern inherited dispositions in the binary layer. One is about the test runner's environment, the other about coverage.
- **`F-25` and `F-28`.** Whatever ruling `F-25` gets decides which failures the concurrency test's retry still has to tolerate. `F-28` should therefore wait for it.

## Rejections

- **`der-3`, the limb "the record does not carry, as a user-visible defect, a lost refusal that the shim's read order cannot exhibit".** Rejected because the evidence falsifies it.
  - The shim does exhibit the loss, on its send rather than on its read: integrity reproduced it with the real `bridge.py`, and reviewer with the shim's sequence.
  - der's own probe shows a send after the close fails, and the shim's order makes such a send possible. The rest of `der-3` stands and is in `F-28`.

No other claim was rejected.

## Tiers

Every tier is the reporting pass's own. Each was checked against `review-findings.md` §Tier, by consequence:

- **`F-25` is Tier A.** A correctly installed bridge's container sometimes gets `Broken pipe` where it should get the refusal. That includes the refused-UID message, the one whose `--allow-uid` remedy `README.md` §Access errors sends users to. The behaviour is older than this branch. Tier is still judged by consequence, not by when the behaviour began.
- **`der-3`'s Tier C** applied to its own claim, a wrong premise in test machinery. It does not conflict with `F-25`'s Tier A, which concerns a different property.

## Findings

Canonical ids are allocated separately for each prefix. Before this round the register's highest ids were `F-24`, `Q-1` and `I-3`. This round mints only `F-` ids, `F-25` to `F-34`. They are written into [`00-index.md`](../../00-index.md) in the same commit as this summary, so the mapping below is canonical.

| Local                                               | Canonical | Tier | Claim                                                                                                    | Route                         |
| --------------------------------------------------- | --------- | ---- | -------------------------------------------------------------------------------------------------------- | ----------------------------- |
| reviewer-1, integrity-1, cleanup-2 (register limb)  | F-25      | A    | A refusal, or the shutdown turn-away, is lost when the peer's send lands after the close, and the shim sends first | architect (`D-11`; host or shim) |
| der-1, reviewer-2, integrity-3                      | F-26      | B    | The termination-signal binary test fails a correct bridge when the runner inherited SIGHUP or SIGINT as ignored | architect (`D-2`, `D-11`)    |
| integrity-2                                         | F-27      | B    | Two of `D-11`'s shutdown-ordering properties have no witness; reversing either passes the suite           | implementer                   |
| reviewer-3, der-3 (mechanism limb), cleanup-2 (scope limb) | F-28 | C    | The concurrency test's retry gives the wrong mechanism and tolerates any I/O error                       | implementer, after `F-25`     |
| der-2                                               | F-29      | C    | The accept loop's retries rest on a TCP premise, and the `WouldBlock` arm's real job is not stated        | architect (`D-11`)            |
| integrity-4                                         | F-30      | C    | Only SIGHUP's inherited ignore is witnessed, and `README.md` step 2 names it as the only exception        | implementer                   |
| cleanup-3                                           | F-31      | C    | `serve_ignoring` takes a signal and arguments its only caller fixes                                     | implementer, with `F-30`      |
| der-4                                               | F-32      | C    | `documentation.md` §5.1's model comment states the stdin-writer constraint that `D-4` §Remediated removed | owner of `documentation.md`   |
| integrity-5                                         | F-33      | C    | The folder README's status still says phase 5 is next                                                   | record owner                  |
| cleanup-1                                           | F-34      | C    | `set_x11_text`, and the seam in `sync.rs`, take an owned `Vec<u8>` that nothing consumes                 | implementer                   |

The full problem report for each finding is in the pass report its local id names. The register entry summarises it and states the required property.

## Routing

**Architect.** These are contract questions the consolidator may not settle.

- **`F-25`.** Does `D-11`'s "gets an error response" require delivery to a peer that sends before it reads? If it does, who owes it? The host could stop closing a stream whose request is unread, the shim could read the response after a failed send, or both could change. `bridge.py` is outside the plan's scope.
- **`F-26`.** How may a binary test control the signal dispositions of the `serve` it starts, given `D-2`'s constraints (only `/bin/sh` and coreutils, no dev-dependency) and the workspace's denial of `unsafe_code`? Alternatively, should the suite state default dispositions as a precondition and fail saying so? Reviewer notes that a POSIX shell cannot reset a signal that was already ignored when it started.
- **`F-29`.** `D-11`'s statement of the `accept` retries.

**Implementer.**

- **`F-27`, `F-30`, `F-31` and `F-34`.** Each can be fixed without deciding anything.
  - `F-27` and `F-30` each allow either a test or a statement that the property is not witnessed. Choosing between them does not change a contract.
  - `F-34` changes a seam's type inside `sync.rs`. Cleanup flagged that as the owner's call, and it is passed on with the finding.
- **`F-28`** follows `F-25`'s ruling.

**Record owner.** `F-32` and `F-33`.

## Notes on the round

- **The journal's note both misplaced the loss and understated it.** It put the loss on the read, observed only under parallel load. The passes found it on the send, and without load. `F-25` and `F-28` both trace back to that note, so an owner sizing the work from the journal would have looked at the wrong half of the exchange and judged the problem rarer than it is.
- **Rulings that put a decision in place keep leaving its tests unchanged.** `der-1` and `der-2` are both survivals from `D-7`. One is a test that still expects `D-7`'s unconditional property. The other is a retry whose reason `D-11` restated from `D-7` without testing it. Round r1's backward pass also accepted the "vanished connection" premise without a probe. This suggests checking, wherever a decision is superseded, the tests and comments that its §Implemented clause named.
