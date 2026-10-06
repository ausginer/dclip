# Finishing a unit of work

> Retrieved when finalizing a unit of work — before formatting, linting, testing or committing.

## Verify what you changed

Cargo commands run from the checkout root and reach the whole workspace. The
order below is the loop you owe every changed file, and each step is cheap
enough that skipping one only defers the failure.

- format with `cargo fmt`. The repository's [`.rustfmt.toml`](../../.rustfmt.toml)
  is the whole of the formatting configuration. **One tool decides formatting,
  and a lint may not re-decide it** — `cargo fmt` is the authority, and a clippy
  lint that reports formatting is a second formatter. A formatting diagnostic
  surviving `cargo fmt` is a defect in the gate rather than in the file: report
  it as one, and do not hand-edit the file to satisfy the linter against the
  formatter.
- lint with `cargo clippy --workspace --all-targets -- -D warnings`. Fix what it
  reports. If a lint is wrong for the code, `#[allow]` it **at the narrowest
  scope with a comment saying why** — never crate-wide, and never by loosening
  the invocation.
- test with `cargo test --workspace`. This is the default run, and it must stay
  correct on a machine with nothing provisioned — no Wayland session, no
  `wl-paste`, no `xsel`. A test that needs one is `#[ignore]`d and run by hand
  when the change touches what it covers;
  [`test-architecture.md`](test-architecture.md) says when such a test is owed.
- test the container shim with
  `python3 -m unittest discover -s . -p test_bridge.py -v` whenever `bridge.py`
  or `test_bridge.py` changes, and whenever the socket protocol changes on
  either side — the host and the shim ship together, so a protocol change is
  owed by both suites.
- test the effort guard with
  `node --test .claude/plugins/harness-effort-guard/tests/` whenever the plugin,
  `.claude/`, `.claude-plugin/` or a role definition changes.
- check every changed shell script with `bash -n`.

**The crate has no features.** If one is added, every composition it creates is
a separate test run, because a suite gated on a feature the default run does not
enable compiles to nothing and reports `ok`. A composition is what the resolver
resolved, not what the flag asked for, so read it back:

```sh
cargo tree -p devcontainer-clipboard-host -f "{p} feats={f}" | head -1
```

Markdown this repository owns — `AGENTS.md`, `CONTRIBUTING.md`, `README.md`,
`.agents/docs/`, `.plan/` — has no formatter. Match the surrounding file, and
keep relative links resolving.

Two things that catch people out:

- **`cargo check` is not `cargo clippy`.** A change that compiles can still carry
  a lint the gate will reject, and finding that out at commit time is the
  expensive order to find it out in.
- **Adding a dependency is `cargo add <name>`**, from the crate directory, rather
  than an edit to `Cargo.toml`, so the version is resolved rather than guessed.

## Commit the finalized state

**Commit every finalized handoff state without waiting to be asked.** A state is
finalized when your unit of work is complete and ready to hand to the next role
or to the user:

- an architect commits the completed contract or plan before handing it to an implementer;
- an implementer commits the completed implementation before review;
- a reviewer commits the completed review artifact before handing findings back;
- remediation is committed once that remediation pass is complete.

- Commit only changes belonging to the completed unit. Never sweep unrelated work into the commit; stage paths deliberately when the working tree holds anything else.
- If there is no tracked diff, do not create an empty commit.
- Use a **short, subject-only** commit message unless a body is explicitly requested. Never copy completion reports, test output, review summaries or plan prose into it.
- Describe the substance of the completed unit, not its workflow position: prefer `host: reject oversized requests` over `phase 2`, `address review` or `finalize implementation`. For a review or planning artifact, name what the artifact establishes or evaluates rather than the checkpoint it belongs to.

## Publish the branch

**A finalized handoff is pushed, not only committed.** Once the unit's last commit is in, `git push` the current work branch to `origin`. Where the unit took several commits, push once after all of them rather than after each — the next role needs the finished state, not the intermediate ones.

Push the work branch and nothing else. Opening or merging a PR, force-pushing, pushing `main`, and moving any other shared ref all still require being asked, as do rewriting history and working around branch protection; those restrictions are resident in `AGENTS.md` and apply from the start of a session, not only here.

If the push is rejected — protected branch, non-fast-forward, missing upstream — report the blocker. Do not force, and do not reshape history to make it land.

## Change record

What this document used to say, and what changed it. No entries yet.
