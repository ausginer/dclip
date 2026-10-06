# Agent workflow

How the multi-agent system is arranged: which roles exist, how a review round is run, and how the roles are configured. This is coordinator and human documentation. **No role reads it at runtime** — each role's own definition under `.claude/agents/` is the executable truth for what that role does, what it loads, and what authority it holds.

## Roles

| Role           | Owns                                                                                          |
| -------------- | --------------------------------------------------------------------------------------------- |
| `reviewer`     | Feature proof — implementation against the plan, contracts, tests and parity requirements     |
| `integrity`    | Crate coherence — neighbouring flows, public surface, invariants, drift outside the change    |
| `cleanup`      | Code discipline — machinery the code's responsibility does not require                        |
| `der`          | Surviving justification — machinery resting on a decision or assumption that may have expired |
| `consolidator` | Synthesis — validates, deduplicates, merges, rejects, routes. The root console                |
| `architect`    | Decisions that need architectural, contract, parity or public-surface authority               |
| `implementer`  | Implementation within settled constraints                                                     |

For important checkpoints, an independent model may be used instead of the Claude `reviewer` role.

Two boundaries hold across the whole system and are stated in each role that they bind: **reviewers find and document rather than fix or decide**, and **only the architect creates, amends, supersedes or renumbers a `D-*`.** Any role may report one as expired, contradicted or unimplemented.

## Dispatch

**Work is driven by direct role sessions.** The owner starts one session per role — `architect` when architectural work is needed, `implementer` for settled implementation, a fresh `consolidator` for a review round, or a fresh `reviewer`, `integrity`, `cleanup` or `der` for a single lens — and coordinates them personally. Nothing routes between the owner's prompt and the role, and there is no tracked main-thread default.

**A session selects its effort as well as its role.** `--agent <role>` applies the definition's `model:` and does **not** apply its `effort:`, so the level is the starter's to pass. [`.scripts/claude-role.sh`](../../.scripts/claude-role.sh) makes both selections from the definition; the guard denies the first tool call of a session that got it wrong, and repairs nothing. The mechanism and what has been measured about it are in [`harness-effort-guard.md`](harness-effort-guard.md) §Starting a role session.

**A consolidator spawns the review lenses itself**, which is depth 2 and observed on the same terms as depth 1. That one dispatch edge is enforced rather than only written down: the guard refuses a dispatch in which `consolidator` selects a role its topology does not allow, `general-purpose` included, before the child starts. The effort invariant cannot catch that on its own — a generic child resolves out-of-domain, which allows. See [`harness-effort-guard.md`](harness-effort-guard.md) §Governed dispatch topology.

**Nothing watches a session's size, and nothing replaces one.** A conversation is disposable working memory; durable state is the repository — commits, contracts, `D-*` records, plans, review artifacts and handoffs, as [`AGENTS.md`](../../AGENTS.md), [`review-findings.md`](review-findings.md) and [`handoff.md`](handoff.md) define. **Anything a session knows that is not in the repository is lost when it ends, by design**, so an owner ends a session at a boundary that coincides with a record being written:

| Session                       | End it when                                               |
| ----------------------------- | --------------------------------------------------------- |
| `implementer`                 | the unit of work is committed and pushed                  |
| `architect`                   | the contract, plan or phase it was reasoning about closes |
| `consolidator`, review passes | always — every round is a new session                     |

**Dispatch happens from the main checkout only.** A linked worktree carries its own `.claude/` at its own commit, so it can govern part of the role set and allow the rest, or govern all of it at a superseded generation. A worktree-rooted session may still run as a single worker — that is what worktree isolation is for — but it dispatches nothing. A session carrying `CLAUDE_CODE_EFFORT_LEVEL` cannot act at all: the guard denies its first tool call, and the remedy is to remove the variable and restart. Both gates are resident in [`AGENTS.md`](../../AGENTS.md) §Before running a governed role.

## Handoff

```
implementation → independent passes (parallel, isolated) → consolidation
              → decision (only when needed) → remediation → closure review
```

Not every finding needs the architect; straightforward defects go from consolidation to the implementer.

## Review round

The consolidator is the root console: it launches the passes, in parallel and isolated from each other, and then synthesises them. **The rules it launches under — which passes run at which boundary, that no pass receives another's report, and that the passes are parallel sub-agents rather than a team — are stated in [`consolidator.md`](../../.claude/agents/consolidator.md), which is the role that applies them.**

The reason behind the isolation is the part worth knowing here: a pass that sees another's findings stops being a second opinion.

What a pass produces — the report shape, the artifact path, the local ids and the tier vocabulary — is in [`review-findings.md`](review-findings.md), which the passes retrieve themselves.

## Prompts

Keep prompts as short as possible. The role definition and the documents carry the context; a prompt identifies the task and the role.

`Could you please review the project against plan.md as part of Checkpoint D?`

`Could you please analyze C2-01 in checkpoint-d-2.md?`

## Agent configuration

Model and reasoning effort are frontmatter in `.claude/agents/`, which is the executable truth. The principle behind them: spend reasoning on synthesis and on lenses that must form a hypothesis nobody wrote down; a lens applying a written rulebook runs cheaper.

Roles keep normal local development tools. A reviewer's `disallowedTools` removes the most obvious edit path and **guarantees nothing** — `Write` and `Bash` remain. That a review changed no production code is established by the diff against the pre-round baseline.

**Three harness properties worth knowing.** A sub-agent inherits the resident instruction set — `CLAUDE.md` and `AGENTS.md` — with no supported opt-out, which is why the rulebook and every convention are referenced rather than resident and are named by the roles that need them. A bare `@path` line is expanded only in those resident files: **inside a role definition it stays literal text**, so a role states its dependencies as read instructions rather than imports. And a new or changed role definition, like a changed resident file, does not take effect in a session that is already running: the stale version persists for minutes rather than resolving on a useful timescale, while a freshly started process picks the change up immediately. Anything that depends on a role's definition or on the resident set is therefore verified from a new session, never from the one that changed it.
