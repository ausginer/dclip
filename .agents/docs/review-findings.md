# Review findings

> Retrieved before writing anything into a review report, including a report with no findings.

What a finding is, and how a review pass reports it.

## Problem reports

- Finding
- Current behavior / contract
- Why it is a problem
- Evidence / reproduction
- Required property

Describe **what is wrong and what property must hold**, not how to fix it. Avoid putting proposed fixes into review documents unless choosing the fix is itself the task.

## Artifacts

`.plan/reviews/<round>/<topic>-<author>.md`; consolidation as `<round>-summary.md`.

**While a round is running, each pass is alone.** A pass writes its artifact to a staging location of its own, outside the shared round directory and shared with nothing. **It may not enumerate, search, read or cite another pass's artifact or staging area while any pass is still running** — not a directory listing, not a search across the tree, not a citation. The consolidator publishes every artifact into the round directory once every independent pass has terminated, and not before. A round directory that fills up while passes are working is the coupling parallel passes exist to prevent, wearing a filesystem.

**A pass that observed another's work has lost the half of its report independence was buying.** Its negative coverage and its silence are inadmissible: an area it calls clean and an area it says nothing about were both reached with another lens's answer already in hand, and neither can be told apart from agreement. Both are rerun, or recorded as unreviewed. Its positive findings stay reviewable where their own evidence supports them, because a defect with evidence is a defect whoever pointed at it first.

A report carries three things, because the consolidator decides with each of them:

- **the state files were read in**, named by the round and never by a commit — [`documentation.md`](documentation.md) §10. Reports from different trees cannot be merged, so a pass reports the commit it was launched on in its closing message to the consolidator rather than in its artifact, and the consolidator confirms that every pass read the same one before merging anything;
- **scope** — what the pass covered and what it did not, so a silent area is distinguishable from a clean one;
- **findings** — each with a reviewer-local id (`cleanup-1`, `integrity-3`), a tier, a one-line claim, and its evidence.

A null result is a result and is stated explicitly. _The forward pass found no surviving machinery_ is an outcome; silence is not.

**A summary proposes canonical ids; the register assigns them.** `.plan/00-index.md` is the one register and allocates the `D-`/`F-`/`Q-`/`I-` series. The ids are hand-numbered, so parallel passes would race: each pass numbers within itself, and the summary resolves the race by carrying a `Local → canonical` mapping. **That mapping is a proposal until the register carries the rows**, and the registered id wins a collision — so whoever mints an id writes its entry into the register **in the same commit that first uses it**.

**A finding is homed by the property, not by the evidence.** A defect in the socket protocol that `bridge.py` exhibits and the host causes is one finding about the side that owns the violated property; where each side fails a property of its own and each is fixable alone, it is two findings.

**A summary's headings must not claim the ids they propose.** `#### F-12` and a pass-local `### reviewer-1` both open with something matching `[A-Za-z][A-Za-z0-9]*-\d+`, so both claim it wherever they sit. Keep the identifier out of the heading's opening position, or put the local name outside the grammar — a letter after the hyphen, as `TE-D12` does.

## Tier

**Tier is assigned by consequence.** Never by provenance, and never by how many lenses reported it.

| Tier  | What it means                                                                                                                                                         |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A** | A correctly installed and configured bridge behaves differently at runtime: what a container can read, what the host exposes, an error, a hang, or the state left behind |
| **B** | No runtime behaviour changes, but a user following the documentation can be misled by it, **or** an instrument the repository relies on is unsound                     |
| **C** | Internal only: no user-observable effect, and nothing the repository relies on depends on it                                                                          |

**An instrument is anything the repository consults instead of re-deriving a fact** — a test, a probe, or a table in the record listing which sites or tests hold a property. Such a table is unsound when a cell is wrong, so a finding against a cell is filed on the second limb of **B** unless nothing relies on the table.

A finding that is _systematic_ rather than isolated does not change tier — it changes priority **within** one.

## Change record

What this document used to say, and what changed it. No entries yet.
