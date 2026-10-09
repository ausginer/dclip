# Documentation model

A document's home is decided by **what makes it wrong**, not by what it is about.

Four things go wrong in four different ways, and mixing them is what this model exists to prevent: a policy a reader must diff against itself to extract the current rule, a coding convention living in a vendor's agent file in two drifted copies, and source comments in which decision narrative outweighs the code they describe.

| Kind           | Answers                                  | Goes wrong when                         | Home                      |
| -------------- | ---------------------------------------- | --------------------------------------- | ------------------------- |
| **Convention** | What do I write?                         | someone decides to write it differently | `.agents/docs/`           |
| **Policy**     | What may I spend, and what must I prove? | a measurement falsifies it              | `CONTRIBUTING.md` Part II |
| **Operation**  | How do I run things here?                | the tooling changes                     | `AGENTS.md`, `CLAUDE.md`  |
| **Record**     | Why is it like this?                     | never — it is what happened             | `.plan/`                  |

The record is the only one of the four that is **append-only**. The other three are **current-state**: they are rewritten in place, and what they used to say is not their business.

That single asymmetry produces every rule below.

---

## 1. Current-state documents state the current state

A rulebook that keeps its withdrawn rules asks the reader to compute the rule before applying it. That is a cost paid on every read, by every reader, to serve one reader who wanted to know what changed — and that reader has a record.

- **No strikethrough.** A struck sentence is a claim the reader must first determine is false.
- **No dates on rules.** A rule is in force or it is not; when it came into force is a record question.
- **No amendment narrative.** _Corrected on…_, _this section used to argue…_, _sharpened…_ are all the record's voice.
- **No rejected alternatives.** Why the other design lost is the most valuable thing in the record and the least useful thing in a rulebook.
- **No worked examples carrying live numbers.** See §4.

What survives the move is not the argument but its **conclusion**, and conclusions are usually one sentence.

### The one thing the record cannot replace

A rule that a **measurement falsified** must say so, or the same measurement gets commissioned again. The satisfying form is a single evidence line under the rule, naming the finding and linking the record — not the argument, and never the falsified wording:

> Measured: _the finding, in one clause_ — followed by a link to the record.

One line, and the reader who wants the method and the numbers follows the link.

---

## 2. Citations bind section numbers, not sentences

A current-state document is tempted to keep its withdrawn wording inline on the ground that _other documents cite it_. They do not: a citation addresses a section number — `§1.1`, `§4`, `§13` — and struck text does nothing for it.

So the compatibility obligation a current-state document actually carries is **numbering stability**:

- **A section number is permanent once anything cites it.** Numbers are never reused and never re-sorted. A section whose rule is withdrawn keeps its number and says what replaced it in one line.
- **New rules take the next free number**, even where a lower one would read better.
- **Renumbering is a breaking change** to every citing document and is done, if ever, with the citations updated in the same commit.

---

## 3. One copy, and it is vendor-neutral

Two files holding the same conventions drift, and the drift is silent because neither is wrong on its face.

- **`AGENTS.md` is the durable root.** Vendor-neutral name, read by humans and by every agent harness that can be pointed at a file.
- **`CLAUDE.md` imports it** with a bare `@AGENTS.md` line and adds only what is specific to Claude Code — tool-availability protocol, sub-agent policy, how a role session is started. It states no convention of its own.
- **`.agents/docs/` holds the conventions themselves**, one subject per file, referenced rather than resident and named by the role or reader that needs each one.

### Residency is a real property and costs real context

An imported file is loaded into every agent's prompt whether or not it is needed, and that cost is charged to every role the harness starts — including roles that neither write code nor consult the rule. A referenced file is read only when someone goes looking. So the arrangement is a **routed knowledge base**: a small always-prepended bootstrap, and chunks retrieved by the role that needs them.

- **Prepend only what must fire before a routing decision exists** — restrictions on irreversible or outward-facing operations, and enough orientation for a role-less session to find everything else. A resident index of chunks that roles already name is creep, not orientation.
- **Everything else is a chunk**, named by the role that needs it at the trigger where it becomes relevant: at start, before writing findings, before finalizing, or on a stated condition.
- **Content too small for a retrieval to pay for itself goes inline** in the role that owns it. A retrieval costs a call and a result as well as the text, so below some size a separate file is more expensive than the duplication it avoids.
- **A chunk is the smallest unit never partially needed.** If a role routinely wants half a file, the file is too coarse.
- **The role definition is the only place an edge is stored.** A chunk states its purpose and trigger in a one-line retrieval header; it does not enumerate its readers, because a second copy of the graph is a consistency problem rather than a witness. _Who reads this?_ is derived from the role definitions.

A large document with permanent section numbers is addressable without being loaded whole: extraction terminates at the next heading of the same or higher level, so a stable address never requires stable adjacency.

Imports are resolved relative to the importing file and nest up to four hops. **A bare `@path` line is expanded only in the resident files** — inside a role definition under `.claude/agents/` it stays literal text, so a role states its dependencies as read instructions.

**A path inside backticks is not an import.** That is the mechanism by which this document can name `@AGENTS.md` without loading it, and it is also the trap: un-backticking a path in prose silently adds a file to every agent's context. Write bare `@` lines only where residency is the intent, and keep them together so the resident set is readable at a glance.

---

## 4. A durable rule cites a record; it does not copy the record's numbers

A number has an owner — the instrument that produces it and the pass that re-bases it. A copy of that number in a rulebook has neither, so it goes stale without anything failing.

A rule illustrated by _the release binary measured at 600 kB_ describes one build on one day. Change a dependency and the sentence describes nothing that exists, while the rule it was illustrating is durable and correct throughout.

- **State the rule generically.** _A deliberately tight budget is tight on purpose_ needs no byte figure.
- **Where a figure is the evidence, link it** rather than transcribing it.
- **Anonymised is not the same as durable.** _One dependency carries…_ still goes stale; it just makes the staleness harder to find.

---

## 5. Source comments

**Nothing in this repository has a published API surface.** `claude-clipboard-host` is a binary crate and `bridge.py` is a script, so every comment, Rust doc comment and Python docstring here is a **maintainer note**: its reader is someone changing this code now. The user-facing surface is the command line, the socket protocol and `README.md`, and those are documented in `README.md`.

### 5.1 What a comment states

- Present tense, describing what the code is and what must hold for it to be correct.
- **Preconditions the code relies on and cannot check belong here and are load-bearing** — _the socket directory is owned by the serving user_, _the request line is at most 4096 bytes_. `CONTRIBUTING.md` §1.1 deletes runtime guards on the strength of such sentences existing, so deleting one silently converts a documented boundary into an undocumented one.
- **State a constraint that holds and the consequence of breaking it.** _The lock is released only after the socket path is removed; an instance that unlocks first can delete the socket a successor has just bound_ is a working comment. It stops a specific edit.
- **One bare pointer is allowed** — `(D-7)` — as an index entry into the record. It carries no argument; it says where the argument is.
- **No strikethrough, no dates, no phase numbers, no review-file narration, no vote counts.** A superseded sentence is deleted. If it needs to be preserved, the record preserves it.
- **Argue for what is, never about what was.** A comment may say why the current shape is the right one and what the obvious alternative gets wrong — that is a constraint on the next edit, stated where the edit happens. It may not narrate that the alternative was considered, by whom, or when. State the alternative's property, not the deliberation.
- **Length is not the test.** A rationale is as long as the mistake it prevents is attractive. §5.3 decides these, and it decides them the same way whether the comment is one line or thirty.

### 5.2 User-facing text

Usage strings, error messages and `README.md` carry only what a user outside this repository can act on: no record identifiers, no `§` citations of internal documents, no `.plan/` links, no dates, no commit references.

### 5.3 The test

For deciding whether a sentence is a comment at all:

> If this became false tomorrow, is deleting it enough — or would someone have to be **told** it used to be true?

Deleting is enough → comment. Someone must be told → record entry.

This is the question that separates a design rationale from a decision record, and it separates them by **tense**. _Why the socket is mode 0666 and the UID is checked per connection_ survives a redesign by being deleted; nobody is owed the news. _The response header gained a field and old shims ignore it_ cannot be deleted, because a shim already installed on someone's host is still reading it.

---

## 6. What the record is for

Everything the rules above evict. The record is where the repository's actual reasoning lives, and none of this diminishes it:

- the argument, the alternatives and why each lost;
- the measurement, its instrument, its injection and its numbers;
- the wording a rule used to carry, with the date it changed and what falsified it;
- the finding that reopened a settled question.

**The record is one register.** `.plan/00-index.md` allocates every series — `D-` decisions, `F-` findings, `Q-` questions, `I-` investigations — and each entry is a `####` heading that opens with its identifier: `#### D-1 — title`. Identifiers are bare, because there is one scope. A heading that only mentions an identifier names it in a parenthesis and does not claim it. One entry reads back with:

```sh
awk -v re="^#### D-1( —|$)" '$0 ~ re {f=1;print;next} f && /^#{1,4} /{exit} f' .plan/00-index.md
```

`( —|$)` stops `D-1` answering for `D-12`, and the `#{1,4}` terminator cannot match `#####`, so an entry's own sub-clauses stay inside the extraction.

**A record entry is append-only and dated**, and a superseded entry is amended in place with its supersession named rather than rewritten. The obligation runs the other way from the current-state documents: a record that quietly drops what it used to say has destroyed the only copy.

**A decision is atomic.** A **substantive** amendment — one that changes what the decision requires of the code — mints a new decision that supersedes the old one, and the old one goes inactive. A **non-substantive** edit — wording, a corrected citation, a re-pointed link — is made in place, with no new id. The distinction is what lets a status be read per decision rather than per sentence: a decision amended in place is partly in force and partly not, and nothing can tell a reader which half a given mechanism rests on.

**What an entry requires is its required-property clause.** A decision or settled finding carries an argument and then a list of what is required of the implementation, as properties rather than a shape; that list is the whole of what an implementer owes, and a signature named in the argument is illustration.

**An instrument belongs to the record it measured for, and retires with it.** A probe, fixture or script written to settle an investigation is how its numbers were taken, so it retires when the investigation does, not when the thing it eliminated does. A retained probe is kept out of the workspace's build and names the investigation it serves.

**Nothing may be deleted from a current-state document or a source comment into the record unless the record already carries it.** Where it does not, it is written there first, in the same change.

**A site is addressed by a name the source carries** — a function, type, constant or test, with its path. A line number is decided by every edit above it and goes wrong silently, so it appears only inside a claim anchored to a named state (§10).

**An entry's own disposition lives in the entry**, and the register publishes no second table of statuses beside it: a copy with nothing to refuse when the two disagree drifts.

**A withdrawal inside an entry removes the text and carries it in a dated note.** The one exception is a commit identifier, which §10 removes without carrying. Markdown strikethrough is not used anywhere in the record.

---

## 7. Witnesses

A convention with no instrument is a convention that decays, and the classes above differ in how checkable they are.

| Property                                                    | Instrument                                                                                                                            |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| No record identifier reaches user-facing text               | none available; held by the rule in §5.2                                                                                              |
| Cross-document links resolve                                | none available                                                                                                                        |
| Section numbers are never reused                            | none available; held by the rule in §2                                                                                                |
| The routing graph closes                                    | none available; every file carrying a retrieval header is named by at least one role definition, and every path a role names resolves |
| The guard's three loading files agree                       | `installation.test.ts` under `.claude/plugins/harness-effort-guard/tests/`                                                           |
| No tracked document uses a commit identifier as a reference | none available; held by the rule in §10                                                                                               |

**Report a figure before budgeting it.** A ceiling whose calibrating injection cannot be re-run is not calibrated.

---

## 8. Where things are

| Path                                              | Kind                                                                                                                                                                       |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AGENTS.md`                                       | Operation — the durable root and the only copy; the always-prepended bootstrap                                                                                            |
| `CLAUDE.md`                                       | Operation — Claude Code overlay, imports the root; also prepended                                                                                                         |
| `CONTRIBUTING.md`                                 | Convention and Policy — source shape and the cost and ownership policy. Referenced, not resident: read whole by the roles that apply the whole rulebook, by section otherwise |
| `README.md`                                       | User documentation — building, installing and using the bridge                                                                                                            |
| `.agents/docs/documentation.md`                   | Convention — this document                                                                                                                                                |
| `.agents/docs/review-findings.md`, `handoff.md`   | Convention and Operation — routed chunks; each carries a retrieval header naming its trigger                                                                              |
| `.agents/docs/test-architecture.md`               | Convention and design reference — consulted, not resident                                                                                                                 |
| `.agents/docs/agent-workflow.md`                  | Operation — how the roles are arranged and a round is run. Coordinator and human documentation; no role reads it at runtime                                               |
| `.agents/docs/harness-effort-guard.md`            | Operation — how the guard loads here and what has been measured about it                                                                                                  |
| `.claude/agents/`                                 | Operation — the role definitions, and the only store of `role → chunk` edges                                                                                             |
| `.claude/plugins/harness-effort-guard/`           | Operation — the guard itself; its own `README.md` is the reference for the domain it governs                                                                             |
| `.claude-plugin/marketplace.json`                 | Operation — the repository-local marketplace that declares the guard                                                                                                     |
| `.scripts/`                                       | Operation — role-session launchers                                                                                                                                        |
| `.devcontainer/`                                  | Operation — the development container and its lifecycle scripts                                                                                                          |
| `crates/host/`                                    | Source — the `claude-clipboard-host` binary that runs on the host                                                                                                         |
| `bridge.py`, `test_bridge.py`                     | Source — the container-side `wl-paste`/`xclip` shim and its tests                                                                                                         |
| `setup-host.sh`                                   | Source — the host installer                                                                                                                                               |
| `.plan/`                                          | Record — `00-index.md` is the register; reviews go under `reviews/<round>/`                                                                                               |

This document is itself governed by the model it describes: it states the rules in force and carries no history of its own.

---

## 9. Change record

What this document used to say, and what changed it.

### 2026-10-08 — §10 replaces a commit identifier found in a tracked file, and §6 names the exception

§10 gained the paragraph that begins _"An identifier already in a tracked file is replaced where it stands"_. §6's withdrawal rule gained its second sentence. Before this change, §6 read: _"**A withdrawal inside an entry removes the text and carries it in a dated note.** Markdown strikethrough is not used anywhere in the record."_ Until then the two sections disagreed about an identifier already in the record: §10 forbade it, and §6 would have carried it into a note. `F-19` found the case, and `D-13` decided it.

### 2026-10-09 — §5.1's model comment states a constraint that holds

§5.1's example of a working comment read: _"The child is killed before the stdin writer is joined; joining first blocks forever on a tool that stopped reading"_. `D-4` §Adjudicated found that this constraint did not guard what it claimed to. `D-4` §Remediated then removed the writer thread, so the example described code that no longer existed. `F-32` found it. The new example is a constraint that `D-15` requires, so it can go stale only through a decision that supersedes `D-15`. The rule itself did not change.

---

## 10. Durable references

**A commit identifier is a coordination token, and no tracked file uses one as a reference.** Between tasks that are running now a commit is exactly the thing to pass — the state a consolidator launches its passes on, the commit an implementer hands to a review — and it passes in a prompt, a message or a command. It is not written into a tracked file as an identifier, as evidence, as a status or as provenance. The rule is about what a token names, not how it is spelled.

The reason is how work reaches `main`: a work branch can land squashed, so a commit cited from the branch resolves only while the branch is left undeleted. And an identifier carries nothing a reader can check by reading it, so a wrong one reads exactly like a right one.

**An identifier already in a tracked file is replaced where it stands** by a name from the table below. That holds in the append-only record too. A dated note beside the replacement records that a commit identifier was replaced, and does not repeat it. This is the one withdrawal that does not carry the removed text, because the removed text is the violation, and the record loses nothing a reader could have checked. Evidence that a file holds an identifier describes the token, for example _a range of two commit identifiers_, rather than quoting it.

### What names a change instead

| Name          | Written as                                                    | What it identifies                                                                |
| ------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Round         | round `bridge-review-1` — its directory's name under `reviews/` | a review round, and the state its passes read                                     |
| Landing       | the landing round `bridge-review-1` reviewed                  | the change a round reviewed                                                       |
| Record change | the change that mints `D-3`                                   | a change named by what it did to the record                                       |
| Entry         | `D-3`, `F-7`                                                  | a decision, finding, question or investigation                                    |
| Site          | a function, type, constant or test, with its path             | where the tree carries something, as §6 requires                                  |
| Merged change | `#3`                                                          | a pull request merged to `main`, which resolves from the repository's own history |

A record entry is dated as well, and a date with one of these names is what lets a reader date a claim from the entry alone.

### How a claim is written

- **Implementation.** An entry is shown implemented by its sites and its tests, and dated by the landing that built it.
- **Evidence and measurement.** A claim about the tree names the state it describes — _in round `bridge-review-1`'s reviewed state, the limit check was in `capture`_. A measurement names its instrument and its tree the same way, and is re-taken on the tree in front of the reader rather than trusted across a merge.
- **Another repository's commit** is named by something that repository keeps — a tag, a release or a merged pull request — or by its branch and the date it was read.
- **Any other hexadecimal token** — an agent id, a digest — is qualified by its kind, `agent:<hex>`, `sha256:<hex>`, so no reader takes it for a commit.
