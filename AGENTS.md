# Working in this repository

The durable instructions for anyone — human or agent — making changes here. Vendor harness files live beside this one and import it; they add nothing normative.

Resident in every agent's context, so it carries only what applies before a role knows what work it is doing. Everything else is named by the role that needs it, when it needs it.

## Before you change anything tracked

- **Tracked changes belong on a non-`main` branch.** Verify the branch first; if it is `main`, stop and ask. Do not create or switch branches implicitly unless asked.
- **Do not amend, squash, rebase, rewrite or delete an existing commit** unless asked.
- **Push the current work branch to `origin` once a unit of work is finalized** — after the last commit of the unit, not after each one. **Do not open or merge a PR, force-push, push `main`, or rename, delete or otherwise move any other shared ref** unless asked.
- **Never bypass branch protection, rulesets or repository policy.** If an operation is rejected, report the blocker rather than force-pushing or changing rules around it.

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before writing or changing source. Before finalizing a unit of work — formatting, linting, testing, committing or pushing — read [`.agents/docs/handoff.md`](.agents/docs/handoff.md).

## Before running a governed role

The effort guard loads from project settings, and **a fresh checkout's first session runs before it is loadable**: registering the repository marketplace and loading its plugin happen on successive starts. A role acting in that window is unchecked, and an unchecked session is indistinguishable from a clean one, because nothing is watching.

A guarded session says so. `Effort guard active` arrives as startup context in every session the guard is loaded into, and it says so whether or not the role definitions resolved — the string answers _is the guard loaded_, and a resolution failure is reported as a separate sentence beside it.

- **Seen it — work normally.**
- **Not seen it — run `claude plugin marketplace add ./` and start a new session before doing governed work or spawning any worker.** A restart is required either way: a plugin does not become loaded in a session already running.

**This binds every governed role**: `architect`, `implementer`, `consolidator`, `reviewer`, `integrity`, `cleanup` and `der` — the set the guard governs, not a subset of it.

**A role session selects its effort as well as its role.** `--agent` applies the definition's model and not its `effort:`, so the level is the starter's to pass; the guard denies the first tool call of a session that got it wrong, and repairs nothing. [`.scripts/claude-role.sh`](.scripts/claude-role.sh) reads the declared level out of the definition and passes it.

**Dispatch only from the main checkout.** A linked worktree carries its own `.claude/` at its own commit, so it can govern part of the role set and allow the rest, or govern all of it at a superseded generation. Where the guard is loaded there it refuses dispatch outright; where it is not loaded there is no gate string to see, and this rule is the whole of the protection. A worktree-rooted session may still run as a single worker — that is what worktree isolation is for — but it dispatches nothing.

[`harness-effort-guard.md`](.agents/docs/harness-effort-guard.md) §Starting a role session is the procedure; §How it loads is the mechanism.

## Evidence

Verify cheap mechanical claims yourself, in the agent that cites them. **Delegate discovery, never verification** — a sub-agent returns candidates; the citing agent confirms the ones it reports. Delegate only when the search justifies a separate context.

Probes, diagnostics, fixtures and benchmarks are fine for any role that needs one to establish a fact. A throwaway Rust probe belongs in a `#[test]` or an example rather than in the host binary; TypeScript, which the effort guard under `.claude/plugins/` is written in, runs directly — `node my-file.ts` — with no build or loader step.

## Where things are

- [`README.md`](README.md) — what the bridge is and how a user builds, installs and checks it.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how code is written: source conventions in Part I, the cost and ownership policy in Part II. This is a small tool — a static Rust binary on the host and a stdlib-only Python shim in the container — not a library: write for a truthful fellow developer who would prefer better performance and a smaller build over defensive checks against invalid usage, and keep the socket peer outside the trust boundary.
- `crates/host/` — the host binary, `claude-clipboard-host`; `bridge.py` and `test_bridge.py` — the container shim and its tests; `setup-host.sh` — the host installer.
- [`.agents/docs/`](.agents/docs/) — conventions and operational references: the documentation model, the test layers, how a review round is reported, the finishing loop, and how the effort guard loads.
- `.plan/` — the record, once it exists: decisions, findings, reviews, measurements, and why anything is the way it is. `.plan/00-index.md` is its register.
