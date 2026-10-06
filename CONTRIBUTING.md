# Contributing

The single source of truth for **how code is written here**: source conventions in Part I, the cost and ownership policy in Part II. Where a document is supposed to live is in [`documentation.md`](.agents/docs/documentation.md).

**Part II's section numbers are permanent.** Records cite them, so they are never reused and never re-sorted, and a section whose rule is withdrawn keeps its number. This document states the rules in force; what it used to say is in the [Change record](#change-record).

**Some rules were measured and some are priors.** Where a rule was tested, this document says so and links the record. A rule with no measurement beside it is a default to be argued with, not a finding.

## What is being written

Two programs and one protocol between them:

- **`crates/host`** builds `claude-clipboard-host`, a static `x86_64-unknown-linux-musl` binary that runs on the Fedora host. It is the only party that touches the real clipboard: it serves image reads over a Unix socket and, with `--sync-text`, mirrors plain text into X11 on the host.
- **`bridge.py`** is a stdlib-only Python shim that runs inside the devcontainer as `wl-paste` and `xclip`. It translates their arguments into a request and writes the image bytes it gets back.
- **The protocol** is one JSON request line, then a JSON header line and the raw payload. Both ends ship together through `setup-host.sh`, so it has exactly one version.

## Reading one entry of the record

The record lives under `.plan/`, and `.plan/00-index.md` is its register: each decision, finding, question and investigation is a `####` entry there, with a bare identifier — `D-1`, `F-2`, `Q-3`, `I-4`. With no tooling, one entry extracts standalone:

```sh
awk -v re="^#### D-17( —|$)" \
  '$0 ~ re {f=1;print;next} f && /^#{1,4} /{exit} f' \
  .plan/00-index.md
```

Both guards carry weight. `( —|$)` stops `D-16` answering for `D-163`, and the `#{1,4}` terminator cannot match `#####`, so an entry's own sub-clauses stay inside the extraction.

**Writing one, and the three rules that keep it addressable.** An identifier is `[A-Za-z][A-Za-z0-9]*-\d+`.

- A heading **opening** with an identifier claims it: `#### F-2` or `#### F-2 — …`. It sits at `####`, and no identifier is claimed twice anywhere in the tree.
- `##### F-2 §Specification` is a named **sub-clause** of that entry. It goes below `####` and under the entry claiming the same identifier.
- A heading that _mentions_ an identifier does not claim it. A document analysing an entry it does not own names it in a parenthesis — `### Socket path is derived (F-2)` — and opens with **Canonical entry: `F-2` — …**.

**An entry's stable anchor is its identifier, never a heading fragment.** A GitHub anchor is the slug of the whole rendered heading, so `00-index.md#f-2` addresses nothing on `#### F-2 — The socket must…` and breaks again on the next retitling. Link the document and write `F-2` for the entry.

---

# Part I — Code style

## Priorities, in order

1. **Correctness and containment of the bridge.** The container gets the current image and nothing else: no clipboard writes, no text over the socket, no unbounded read, no unreaped child, no connection from a UID nobody allowed. Nothing below trades against this.
2. **Runtime performance** — a paste is interactive, so the request path stays short: one listing, one read, no copies of the payload beyond what the framing needs.
3. **Cost** — compiled size, dependency weight and compile time are part of the experience of building and installing this. Keep them minimal unless it costs runtime performance.
4. **Readability** — code must be maintainable. DX does not prevail over correctness or UX. A comment is sometimes better than a less performant but cleaner implementation.

Part II is senior to Part I wherever the two meet.

## Language and platform

- **Rust 2024**, and the formatter is the repository's [`.rustfmt.toml`](.rustfmt.toml). `cargo fmt` decides formatting; nothing else does.
- **The host is a static musl binary**, so it runs on Fedora whatever glibc it has. A dependency that needs a C toolchain or links glibc breaks that property, and taking one is an architectural decision rather than a dependency choice.
- **Prefer `std` wherever `std` covers the use case.** A dependency is a supply-chain surface, a compile-time cost and a semver obligation, and most small crates buy none of that back. `libc` (sockets, signals, process groups) and `serde_json` (the protocol's JSON lines) are the two the host carries.
- **A dependency is added with `cargo add`**, from the crate that needs it, so the version is resolved rather than guessed.
- **`bridge.py` uses the Python standard library only.** It runs with whatever Python 3 the container already has, from a read-only mount, with nothing installed beside it.
- **Shell scripts are bash with `set -euo pipefail`**, and are re-runnable: a devcontainer reruns its lifecycle hooks against state a previous run left behind.
- **Take the narrowest thing that works**: `&str` over `&String`, `&[T]` over `&Vec<T>`, `impl IntoIterator` over a concrete collection where the caller would otherwise build one to throw away.
- **Derive rather than implement** — `Debug`, `Clone`, `PartialEq`, `Default` — unless the hand-written form carries semantics the derive would get wrong.
- **No `async`.** The host serves a handful of short requests with a thread each and a bounded count; an async runtime would be the largest dependency in the build and buy nothing.

## Types

- **A newtype where a value has an identity.** A MIME type and a socket path are both strings to the compiler; wrap where the value is passed around, not where it is constructed and immediately consumed.
- **Prefer an enum to a `bool` pair.** Two booleans admit four states where the domain has three, and the fourth is the one nothing handles.
- **Lifetimes and generics no wider than their callers.** A parameter generic over `T: AsRef<str>` with one caller passing `&str` is monomorphisation and inference cost bought for nothing (§2.1, §3).

## Option, Result, and what absence means

The check states what the value's absence _means_. The fourth case is the one a mechanical sweep breaks.

- **`is_some()` / `if let` where absence is simply absence.** Where no valid value is special, `if let Some(mime) = …` says everything a match said and reads as _is there one_. This is the common case.
- **Match the variants where they carry different information** — a request `type` that is absent, `null`, the word `image` or a concrete MIME type are different answers, and the code that reads it lists them.
- **`?` over a match that only propagates.** A `match` whose error arm is `return Err(e.into())` is `?` written out.
- **Never convert an exact check into a default over a domain with a meaningful default value.** `unwrap_or_default()` is the trap: on a size it turns "no size" into `0`, which is an ordinary size. Likewise `unwrap_or(0)`, `unwrap_or_else(String::new)` where `""` is reachable, and any `Option<T>` whose `T::default()` is a legal member of the domain.

The rule is about meaning, not byte count. Where both spellings are correct, the shorter one wins on §Priorities; where they are not, the correct one wins.

## Errors

- **The host has no library surface, so an error is a boxed message.** Every error ends as text: in the response header the container prints, or on stderr with exit status 1. Classification exists only where code branches on it — `io::ErrorKind::WouldBlock`, `Interrupted`, `NotFound` — and is matched there.
- **An error names the fault and the offending value** — the tool that failed, the UID that was refused, the argument that was unknown. Explanation and remedy belong in the README and the source — see §1.3.
- **No `unwrap` or `expect` on a path the socket peer, the clipboard or the environment can reach.** A panic in a worker thread drops the connection without a response, and a panic in the accept loop stops the bridge. In tests, and in a case the surrounding code has genuinely made impossible, `expect` carries the reason rather than a restatement of the call.
- **The shim fails with a message and status 1**, never a traceback: `bridge.py` catches the errors its own code raises and prints `<tool>: <error>`.

## Shape

- Prefer early return and `?` to nesting. A function whose happy path is at the innermost indent is a function that reads backwards.
- Keep a `match` exhaustive by listing variants rather than by a `_` arm, wherever adding a variant should make the compiler point at this code. A `_` arm is a decision to ignore the future.
- Name a closure's binding for what it is, not for its type.

## Tests

- **A Rust test module is `#[cfg(test)] mod tests` beside the code it tests.** It keeps crate-private access, so the host publishes nothing for its tests.
- **The built binary is tested under `crates/host/tests/`**, through what a user reaches — its arguments, exit status, stderr, socket and signals — with stand-in `wl-paste` and `xsel` scripts first on its `PATH`. Those tests need nothing beyond `/bin/sh` and coreutils, and add no flag, environment variable or `pub` item to the binary. What decides which layer a test belongs to is in [`test-architecture.md`](.agents/docs/test-architecture.md).
- **Python tests live in `test_bridge.py`** and run with `python3 -m unittest discover -s . -p test_bridge.py`.
- **Each test covers one specific piece of that unit's logic**; do not combine several. Three unrelated assertions in one test function is three cases, not one, and the first failure hides the other two. A table of inputs that all exercise one behaviour — every malformed request is refused — is one case.
- **A test name says what should hold**: it starts with — or at least contains — the word `should`. In Python that is `test_should_…`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_list_only_supported_image_types() {
        /* one behaviour */
    }

    #[test]
    fn should_reject_payload_with_wrong_magic() {
        /* one behaviour */
    }
}
```

**A test needing something the default run cannot start is `#[ignore]`d**, so `cargo test` stays correct on a machine with nothing provisioned — no Wayland session, no `wl-paste`, no `xsel`. The layering, the oracle each layer answers to, and when an end-to-end check is owed are in [`test-architecture.md`](.agents/docs/test-architecture.md).

## Comments and doc comments

Governed by [`documentation.md`](.agents/docs/documentation.md) §5. Comments describe **the code that exists now**.

- Present tense, self-contained.
- Explain a non-obvious invariant, ordering constraint, ownership boundary, performance property, or reason for the current implementation.
- No planning or review bookkeeping — no `D-*`, `F-*`, `I-*`, phase numbers, review names, commit hashes or `.plan` sections. An internal comment may carry one bare decision pointer as an index entry.
- Do not narrate history: no "used to", "was changed", "replaces", "previously", "this was added because". If a past decision holds reasoning still needed to understand the code, restate **the reason**, not its provenance. If removing the historical reference leaves nothing useful, remove the comment.

In short: **argue for what is, never about what was.**

---

# Part II — Code of Cost

How we keep the bridge from costing more than it is worth. The goal is **not code golf**: it is to remove runtime machinery the bridge does not need, move guarantees to compile time, keep optional work optional, and keep the user-facing surface — flags, environment variables, the protocol, messages — clear. A cost pass must make the implementation smaller **without making the contract worse**.

The currencies are compiled size, dependency weight, compile time, allocation and the permanence of a published surface. They do not move together, and a pass that reports one of them is reporting one of them.

## 0. Runtime performance wins

Cost is a secondary goal. If reducing it would make runtime behaviour meaningfully slower, more allocation-heavy or algorithmically worse, **runtime performance wins by default**. Do not trade cost for extra work on the request path, an extra copy of a payload that can reach 64 MiB, or recomputation that was intentionally avoided. Such a trade-off is not an automatic optimization: measure it and report it for owner review.

**A size figure cannot answer a runtime question, and it will look like it did.** If a candidate's real cost is _when it runs_ or _what it allocates_, measure that.

## 1. Delete runtime policy before compressing code

### 1.1 No nannying — trust the operator, never the peer

Do not protect the person running the bridge from mistakes that are already their responsibility, and never extend that trust to the socket.

**Correctness has a domain, and the domain is correct use.** The bridge must be correct when it is installed and started the way the README says; under anything the **socket peer** sends — any bytes, any timing, abandonment mid-request; under anything the **environment** does — a clipboard that changes between listing and reading, a tool that hangs or exits non-zero, a signal at any point, concurrent connections; and under any state **the bridge itself** creates and later reads back — its socket, its lock, its watcher. None of that is negotiable and no cost pass touches it (§13). **Outside that domain the bridge owes nothing at runtime.**

**The operator's contract does not have to be checked at runtime.** It may be stated by the type system, by the README, or by an obvious semantic precondition — _run `serve` as the user who owns the Wayland session_, _mount the directory, not the socket_, _pass a numeric UID to `--allow-uid`_. An operator who ignores a documented precondition has **left** the contract, not found a hole in it, and a natural failure through an existing error path is the right answer.

**So reachability is a gate, not the first of two questions.**

- **(a) the gate — can the invalid state arise despite correct installation and use?** The peer did it, the environment did it, a race did it, or the bridge minted the state itself and must read it back. **If none of those: stop.** Nothing considered later reopens the question.
- **(b) the only justification, asked only of what survives (a)** — the bridge would itself break a property it owns: hand the container something other than a well-formed image, write the host clipboard on the container's behalf, outlive a child it spawned, or replace a path it does not own.

**Fail (a) and there is nothing further to argue.** _Bad configuration could make our internals incoherent_ is not a justification: the internals are incoherent **because** something invalid was fed to them.

**_Naturally_ has to mean the existing error path actually runs** — an unparsable UID returns `Err` where it is parsed, a missing binary fails `install`. Where it does not, the code **succeeds and does the wrong thing**. Run the counterfactual to the end and ask what the bridge is left _doing_, not only what it stops returning — then ask (a) about it.

**Verify the failure before you argue about it**, whether deleting a check or defending one — the justification is a claim about what happens without it, and that claim is executable. A check can describe a failure that **cannot occur at all**, and only running it finds that.

In particular:

- do not validate operator input merely to provide nicer error messages;
- do not validate values already constrained by the type system;
- do not reject harmless shapes just because the implementation prefers a narrower one;
- do not copy or normalize data defensively unless the code must take ownership of it;
- do not add checks for unsupported use that would naturally fail through the existing error path.

**This section decides whether a check exists; §1.3 decides whether its message ships.** _Whose state breaks_ selects the check; _who can reach the condition_ selects the payload.

### 1.2 Prefer types over runtime guards

If a constraint can be made impossible or visible at compile time, prefer that over a runtime check — a newtype instead of a validated `String`, an enum instead of a checked discriminant, a guard that owns a resource instead of a later cleanup call. The host's `Drop` guards for its socket, child processes and connection count are this rule applied: cleanup happens on every path because the type makes it so. Do not pay runtime work to enforce a rule the compiler already enforces.

**A constraint the compiler cannot state is still a constraint.** Write it where the operator or the next maintainer will meet it — the README, the usage line, a comment at the site — and note that under §1.1 writing one down is what puts the input **outside** the contract, so the statement is the alternative to the runtime check, not a companion to it.

### 1.3 Do not ship verbose diagnostics by default

Long diagnostic strings are runtime payload: they sit in the binary's read-only data, they are formatted at the point of failure, and they are easy to under-rate.

**A shipped message is an identity, not a narrative.** It names the fault and interpolates the offending value. Explanation, remedy and restatement of the rule belong in the README and the source. A one-clause remedy is the exception where it is the only thing the reader can act on from where they stand — `add --allow-uid 1001 on Fedora` is read inside the container, where the README is not.

**Gate by provenance** — what must be true for it to fire — never by who happens to receive it:

- **only this code's own defect can produce it** → `debug_assert!` it;
- **someone outside can trigger it** — the peer, the operator, the environment → **ship it**.

## 2. Be suspicious of abstraction that exists once

### 2.1 Single-use functions are inline candidates

If a function has one call site, seriously consider inlining it. The same applies to wrappers, builders, adapters, one-off helper structs, functions that merely rename another call, and functions that construct a value only for the callee to immediately destructure it.

Do **not** inline automatically when the function is a meaningful semantic or invariant boundary — `respond_with` exists once in production and is the seam that makes the protocol testable without Wayland. If keeping a single-use abstraction materially improves the design, record it for owner review.

**A generic with one instantiation is the same finding**, except where the type argument is the test seam.

### 2.2 Do not build a framework to remove duplication

A generic runtime mechanism is not automatically cheaper than two direct paths. Prefer the smallest structure matching the actual extension contract, and be especially suspicious of registries, trait-object dispatch introduced only to look extensible, dynamic handler maps and runtime discriminators introduced only to make code look generic. The protocol has two operations and the shim two tools; static code is the right shape for both.

## 3. Do not pay runtime for type-level architecture

Type-level structure should disappear from the compiled output wherever possible. Prefer zero-sized marker types, `const` generics and enums over runtime tags, compile-time monomorphisation over dynamic dispatch **where the set is closed**, and direct structural use over identity wrappers that exist only to be unwrapped.

The reverse is also a cost: **a generic instantiated over many types is many copies of the same code**. `cargo llvm-lines` is the instrument that shows this.

## 4. Published values are governed by surface and permanence

The bridge's published values are the protocol's fields and operations, the command-line flags and subcommands, the environment variables (`CLAUDE_CLIPBOARD_SOCKET`, `CLAUDE_CLIPBOARD_TRACE`), the install path and the mount point. Do not add one merely to give an internal value a name.

- **(a) Surface.** Every published value is one more thing a user may believe they must understand, paid for in the README and in questions.
- **(b) Permanence.** A value written into someone's compose file, devcontainer or `config.kdl` keeps arriving after the code stops meaning it. Removing it breaks their setup silently or loudly, and neither is free.

**The test:**

> Would a user have to **write this value** to do something the bridge supports?

If yes, publish it, document it in the README, and accept that it is frozen from that day. If no, keep it a private constant or a local literal. A test seam is not a reason to publish a value.

## 5. Do not store what can be derived cheaply

Duplicate state costs code as well as memory. Avoid keeping multiple representations of one fact: a flag derivable from an existing state, a cached copy of a value already owned elsewhere, a counter whose meaning is already encoded structurally.

Prefer one authoritative representation and derive cheap facts from it. Do not replace O(1) state with expensive recomputation to save a few bytes.

## 6. One mechanism, but not one framework

When two paths differ only slightly, prefer one shared mechanism with a small point of variation — `capture` serves `wl-paste` and `xsel` alike — but do not introduce generic machinery solely to deduplicate a few statements. This is a measurement question, not a style rule.

## 7. Keep optional work optional

Local source size is not build size. A helper can make one module smaller while forcing a default build to compile code it previously did not need. The host has no cargo features today; if one is added:

- **optional features should pay for themselves**, and an optional dependency is `optional = true` behind a feature rather than a default;
- **the default feature set is a decision**, not the union of everything that seemed useful;
- **the crate must build with `--no-default-features`**, and that is the composition where feature leakage shows up.

Runtime-optional work — `--sync-text` — costs nothing when it is off: no watcher, no `xsel`, no thread.

## 8. Do not preserve compatibility for an unreleased surface

Nothing is released, and the two ends of the protocol ship together. Do not keep deprecated flags, alias subcommands, legacy request shapes or a protocol version field "just in case". Delete the obsolete shape completely, on both sides, in one change.

After a first release this section inverts for the values §4 names: removing one is a decision for the architect rather than a cost pass.

## 9. Copies and intermediate values need an ownership reason

Do not copy defensively by default. A copy is justified when the code must take ownership, snapshot a value across a mutation it does not control, or cross a thread boundary it cannot otherwise satisfy. Otherwise prefer the borrow. **`.clone()` to silence the borrow checker is a design finding wearing a method call.**

Likewise be suspicious of repeated packing and unpacking, of `String` where `&str` reaches, `Vec<T>` where `&[T]` reaches, and `to_owned()` at a boundary that only reads — and above all of a second copy of an image payload.

## 10. Traits and structs need semantics, not prestige

Use a trait when the trait itself provides required semantics: a genuine open set of implementors or dynamic dispatch the design needs. A plain struct or a function is usually smaller and clearer. A trait with one implementor is an interface with no second party; a closure parameter is the lighter seam.

## 11. Do not write source code like a compiler

Do not manually shorten identifiers, hand-unroll, or distort clear control flow for a few instructions, and avoid tricks whose only justification is that the generated code might be shorter. Let `rustc` and LLVM handle inlining, constant folding, bounds-check elimination and dead-branch removal.

A cost pass should primarily remove **semantics and machinery the runtime does not need**, not imitate the optimizer by hand.

## 12. User-facing clarity is not a cost budget

Do not reduce cost by making the user pay the complexity: no cryptic flag names, no merging distinct failures into one message to save a string, no setup step moved from the installer into the README to save a line of shell.

The implementation is the thing being optimized.

## 13. Correctness and lifecycle semantics are fixed during a cost pass

A cost optimization must preserve the accepted contract. If it requires changing what the container can read, failure semantics, the protocol, a published value, signal and cleanup ordering, or observable timing such as a timeout, it is no longer a cost optimization: stop and report it as a separate design finding. Do not silently trade correctness for cost.

**This fixes the contract, and behaviour outside the contract is not part of it.** Deleting a nannying check changes what happens on input the contract already forbids, which is not a failure-semantics change here. The exception is where the contract **fixed** the outcome. If you cannot tell which you have, you have found a contract gap — stop and report it.

## 14. Do not trade runtime performance for cost

Do not introduce new work on the request path, an extra copy, avoidable allocation or worse asymptotic behaviour for a size reduction without explicit measurement and owner approval. When the two genuinely conflict, **runtime performance has priority by default**.

## 15. Measure every meaningful change

"This should be smaller" is not evidence. For meaningful changes record before/after measurements of the release musl binary, `target/x86_64-unknown-linux-musl/release/claude-clipboard-host`, which is where compiled size is actually observed. Where possible record the reason for the delta — branch removed, wrapper inlined, diagnostic string deleted, dependency edge removed, generic de-monomorphised.

The instruments: `cargo bloat` for where the size went, `cargo llvm-lines` for monomorphisation, `cargo tree -e normal` at both ends of a change for the dependency edges the binary compiles, `cargo tree --duplicates` for one crate resolved at two versions, and the release binary's own size for the total. `--duplicates` answers duplication and nothing else: whether an edge was added is read from `-e normal`.

**Four ways a size measurement lies:**

- **Do not add ablations.** Related changes share code and the optimizer folds across them, so the parts do not sum to the whole. Book only a joint measurement of what actually landed.
- **Say which figure governs.** A symbol's size, a crate's contribution, LLVM lines and total binary size disagree, sometimes in direction.
- **Check that the instrument can see the change.** A build that never compiles the affected code reports 0 for any change to it — true, and not evidence.
- **Do not price a change by reading a file.** Build, measure, then read.

## 16. Prefer deleting machinery over rebasing syntax

The highest-value reductions usually come from removing validation, duplicate state, compatibility, wrappers, runtime indirection, shipped diagnostic payload, a dependency, and abstractions existing only for internal neatness. Do those before syntax-level micro-optimization.

**A dependency is usually the largest single item available**, because it brings its own transitive set, its own compile time and its own supply surface — check it before anything in your own source.

## 17. Questionable optimizations go into the report

Do not automatically apply a change whose win depends on making the code materially less obvious. Report it for owner review when it involves deliberate duplication, unusual syntax, removing a meaningful semantic boundary, a performance trade-off, a user-facing trade-off, or a very small gain relative to readability cost. Include the measured delta and the trade-off.

## 18. A budget is an instrument, and its slack is its sensitivity

A size budget exists to make a change visible. Read as a target instead, it stops doing that.

- **A deliberately tight budget is tight on purpose.** Absorbing a small regression into it by re-basing is the single thing it exists to prevent.
- **A figure that does not move is a result.** When a change should touch nothing in the binary, that unchanged number is the evidence it behaved. Say beforehand what it should do.
- **Re-base after a shrink, never during one.** A pass landing under budget ends in a re-base; a pass landing over it ends in a decision.
- **State the re-base trigger before you need it**, as a condition an observer meets rather than a schedule.

---

# Litmus test

For every suspicious piece of runtime code, ask the gate first:

> **Is this state reachable through correct installation and use, or from the socket peer or the environment?**

**If it is not, you are finished** — the code is a deletion candidate and the second question is never asked. Only for a state that survives:

> **What property the bridge owns requires this code to exist at runtime?**

The code is a strong deletion candidate if the answer is any of: "the type system already prevents it"; "only an operator ignoring the README can get here, and the bridge is left holding nothing"; "the failure it describes cannot actually happen"; "this was added for a nicer error"; "this mirrors state we already have"; "this supports a flag or request shape we deleted"; "this only makes the implementation more generic".

A check on what the socket peer sends is never one of these: the peer is the end user, and its input is inside the domain by definition.

# Order of attack

1. remove nannying and redundant runtime validation — **reachability is a gate: unreachable through correct use, stop and delete; ownership is asked only of what survives** (§1.1);
2. remove a dependency that is not paying for itself, and compatibility and dead published values;
3. reduce shipped diagnostic payload to identities, and `debug_assert` what only your own defect can produce;
4. remove duplicate state and unnecessary carriers;
5. inspect single-use abstractions, wrappers and generics with one instantiation;
6. inspect runtime genericity, trait objects and indirection;
7. inspect copies, intermediate values and repeated shape conversions;
8. only then consider local syntax-level simplification;
9. measure every meaningful change;
10. re-base budgets only after the remaining cost is understood and accepted.

# Definition of success

A successful cost pass leaves the bridge:

- behaviorally identical;
- **no slower on the request path unless an explicit measured trade-off was accepted**;
- easier or no harder to understand;
- at least as type-safe;
- **defensive only where the state is reachable through correct use, the peer or the environment _and_ the bridge would otherwise break a property it owns** — both, never either;
- with every shipped diagnostic attributable to something outside the bridge, and every gated one to something inside it;
- smaller in the binary that actually ships;
- with every remaining significant byte attributable to an accepted runtime responsibility.

The target is not the smallest code we can write. The target is the **smallest runtime the contract actually requires**.

---

# Change record

What this document used to say, and what changed it.

### 2026-10-06 — §Tests gains the binary-level layer

Changed by `D-2`, when the layer under `crates/host/tests/` was added. The first rule of §Tests read:

> **A Rust test module is `#[cfg(test)] mod tests` beside the code it tests.** The host is one binary crate with no public surface, so there is no `tests/` layer; what decides how a test reaches its subject is in [`test-architecture.md`](.agents/docs/test-architecture.md).
