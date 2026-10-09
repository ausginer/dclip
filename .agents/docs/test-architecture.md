# Testing architecture

> Retrieved when adding a test layer, or when deciding which layer owes a fact.

Why the tests are shaped the way they are. The actionable rules — how a test is
written, named and asserted — are in [`CONTRIBUTING.md`](../../CONTRIBUTING.md)
§Tests; this document is the model behind them.

The project is two programs and a protocol between them: `claude-clipboard-host`
(`crates/host`) on the host, and `bridge.py` in the container, posing as
`wl-paste` and `xclip`. The host is the authority over what leaves the host
clipboard — images only, read only, at most 64 MiB, to an allowed peer UID — and
the shim's job is to translate tool arguments into protocol requests. The real
Wayland clipboard and Claude Code are components the project does not own.

## The layers

No single layer proves the bridge correct. A change is well tested when the
layers it touches agree.

1. **Rust unit tests** — in each module's sibling `tests.rs`, such as
   `crates/host/src/protocol/tests.rs`, over logic with no I/O: request parsing
   and response selection through `respond_with` with the clipboard passed in
   as a function of a query, MIME magic checks, response framing into a `Vec`,
   and the text-sync sequence with its tools passed in. They are the whole of
   the proof for anything that is a pure transformation.
2. **Rust OS-boundary tests** — also in the module's `tests.rs`, over mechanisms the
   operating system decides: peer credentials on a `UnixStream` pair, framing
   across a real socket, child-process capture, timeout and reaping with `sh`
   and `cat`. They need nothing provisioned, so they run in the default
   `cargo test`.
3. **Rust binary tests** — `crates/host/tests/`, over the built
   `claude-clipboard-host` driven through what a user reaches: its arguments,
   exit status, stderr, the socket and signals. Stand-in `wl-paste` and `xsel`
   scripts sit first on the child's `PATH`. They replace the Wayland session,
   not the bridge, so what these tests prove is the bridge's own lifecycle —
   binding, the lock, replacement of a stale socket, cleanup on a signal, the
   concurrency limit, the text-sync watcher — and the protocol as the socket
   carries it. They need only `/bin/sh` and coreutils, so they run in the
   default `cargo test`. The peer UID check and the per-connection deadlines
   are proved by the first two layers only: a refusal needs a second account,
   and the running server's 12 s phase is too long to wait out.
4. **Python unit tests** — `test_bridge.py`, over the shim's argument
   translation and output, with `request_host` mocked. They prove that each
   supported `wl-paste`/`xclip` invocation becomes the right request and that
   everything else is refused.

Each catches a different fault, and the value is diagnostic: the failing layer
says where the problem is.

**The criterion is what a test must reach.** A crate-private mechanism is tested
beside the code, because the binary publishes no Rust surface a separate test
target could reach. What the binary does publish — its command line, its exit
status, its socket and its response to signals — is reached from
`crates/host/tests/`, and anything only the running process can show belongs
there. Making something public so that a test elsewhere can observe it buys one
test a permanent seam — `CONTRIBUTING.md` §4's test, failed. That covers a
hidden flag or environment variable as much as a `pub` item: the binary tests
find their stand-ins through `PATH`, which is how the binary finds the real
tools.

**Most of the host is reachable by the first layer, and that is a design
property rather than an accident.** `respond_with` takes the clipboard as a
function of a query, and text sync takes its tools the same way, so the whole
request-to-response policy and the sync sequence are provable without Wayland
or X11.
Keep it that way: a change that reads the clipboard directly inside the policy
has made the policy expensive to prove for nothing.

## The end-to-end check, and when it is owed

**The real clipboard and Claude Code are not the project's**, so a mock of them
asserts what the project assumed, which is the thing in doubt. Their real
behaviour is the oracle, and it is checked against a real session:

- **by hand**, with README step 5 — `wl-paste --list-types` and a byte count in
  the container, then a paste into Claude Code — whenever a change touches what
  the host asks `wl-paste` for, the shim's accepted arguments, or the protocol;
- **or by an `#[ignore]`d test** that drives the real tools, run explicitly.
  It is never in the default run, because a fresh checkout has no Wayland
  session, and a suite that fails on absence trains everyone to ignore its
  failures.

Such a test never changes the user's clipboard content unless it put it there,
and restores what it replaced. Everything reachable without the real tools is
proved without them.

## Core principles

### Test observable contracts

Prefer the boundary a caller sees: the protocol bytes on the socket, the shim's
stdout and exit status, the binary's exit status. Reach into an internal
function only when it is the only meaningful target, and never reconstruct
internal state a test is supposed to be proving.

### Use independent observations

**A test must not compare a value against the same computation that produced
it.** Framing a response and parsing it back with the same helper proves the
helper is self-consistent and nothing else.

A test establishes something when the write and the read are independent: a
different entry point, a parse of the bytes rather than a value held in memory,
or an assertion about a property the writer never computed — the header's
`size` against the payload's actual length, the bytes after the newline against
the input.

### Prefer exact assertions

Assert the value, not that a value is present. `assert_eq!` over
`assert!(x.is_ok())`; the specific error over "it errored". A tolerance belongs
only where the underlying system genuinely has one — clock granularity, an
unordered set of MIME types — and it is named in the test where it appears.

### Evidence for a claim about the whole suite comes from a run that reaches it

A mutation argument has two halves — _this change fails these tests_, and _it
fails nothing else_ — and only the first survives a run that stops at the
first failing target. `cargo test` does stop, and it does so silently: the run
reports `FAILED` either way and omits the targets it never built, so a smaller
failure set reads exactly like a smaller failure. Produce the negative half
with `--no-fail-fast`.

This is about evidence, not about the finishing loop. The runs in
[`handoff.md`](handoff.md) §Verify what you changed answer whether the tree is
green, which a fail-fast run answers correctly.

### An assertion's quantity can take a failing value

**A test's stated oracle is one its subject can still produce, and the quantity it
asserts on is one a broken subject could move.** An assertion over a value that is
fixed by construction reports `ok` for the same reason a correct run does, and the
test then measures nothing while reading as coverage.

Ask of the quantity once: _what would have to be true for this to fail?_ Where the
answer is _nothing the subject can do_, either the detector has moved — another
assertion earlier in the test is now what catches the fault — or the coverage is
gone. Say which, in the test.

**Ask it of each clause the test's prose claims**, not only of the assertion as a
whole. Where a comment says a response preserves _both NULs and newlines_, the
assertion beneath it has to be able to fail for each one. Grow the assertion to
the claim, or narrow the claim to what is pinned.

### One behaviour per test

A test function covers one behaviour of one unit. Three unrelated assertions in
one function is three tests wearing one name, and the first failure hides the
other two.

## Oracle hierarchy

Expected values come from different authorities depending on the contract.

| Contract                          | Primary oracle                                      | Notes                                                                 |
| --------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------- |
| A policy the host enforces        | the rule, stated in the test, and the record entry  | The host is the authority, so the rule is the project's own           |
| Protocol framing                  | the header's fields against the raw payload bytes   | Parsed independently, never by the helper that framed it              |
| Image validity                    | the format's published magic bytes                  | PNG, JPEG, GIF and WebP signatures, written literally in the test     |
| Shim argument translation         | a table of invocations and the expected request     | Input and expected request beside each other                          |
| CLI surface                       | the exit status and output of the program           | What a caller can script against                                      |
| A component the project does not own | that component's real behaviour                  | `wl-paste`, `xsel`, Claude Code — not a mock; see §The end-to-end check |

## Failure interpretation

| Failing layer           | Likely fault                                                                        |
| ----------------------- | ----------------------------------------------------------------------------------- |
| Rust unit only          | Request policy, MIME selection, framing or validation                               |
| Rust OS-boundary only   | Socket, credential or child-process handling                                        |
| Rust binary only        | Argument handling, the server's lifecycle — lock, socket path, signals, deadlines, workers — or the watcher |
| Python only             | The shim's argument translation, or the shim's reading of the protocol             |
| Unit tests pass, end-to-end fails | An assumption about `wl-paste`, `xsel` or Claude Code that does not hold  |

## CI policy

**Not installed.** This section states the requirement, not a mechanism that
runs today: there is no workflow, and local verification is what stands behind
every change.

When it exists, every change is gated on `cargo fmt --check`, `cargo clippy
--workspace --all-targets -- -D warnings`, the default `cargo test --workspace`,
the Python suite and the guard's `node --test` suite. Anything needing a Wayland
session is not part of the gate, because a gate that can fail for a reason
unrelated to the change is a gate people learn to re-run rather than read.

---

## Change record

What this document used to say, and what changed it.

### 2026-10-06 — The binary-level layer

Changed by `D-2`, when `crates/host/tests/` was added. §The layers listed three layers, ending with the Python unit tests as the third, and its criterion paragraph read:

> **The criterion is what a test must reach.** A crate-private mechanism is tested
> beside the code, because the binary publishes no surface a separate test target
> could reach. Making something public so that a test elsewhere can observe it buys
> one test a permanent seam — `CONTRIBUTING.md` §4's test, failed.

§Failure interpretation had no row for the binary layer.

### 2026-10-06 — Unit tests in sibling files

Changed by `D-2` and `D-3`, when the host was split into modules. The first layer read:

> 1. **Rust unit tests** — `#[cfg(test)] mod tests` beside the code in
>    `crates/host/src/main.rs`, over logic with no I/O: request parsing and
>    response selection through `respond_with` with an injected clipboard
>    function, MIME magic checks, response framing into a `Vec`, the text-sync
>    filter. They are the whole of the proof for anything that is a pure
>    transformation.

The second began "also beside the code", and the design-property paragraph read "`respond_with` takes the clipboard as a function, so the whole request-to-response policy is provable without Wayland."

### 2026-10-09 — The binary layer's coverage, stated as pinned

Changed by `F-12`, when a mutation of the running server's admission and of its phase length each survived the suite. The third layer's coverage sentence read:

> not the bridge, so what these tests prove is the bridge's own lifecycle —
> binding, the lock, replacement of a stale socket, cleanup on a signal,
> deadlines, the text-sync watcher — and the protocol as the socket carries
> it.
