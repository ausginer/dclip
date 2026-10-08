# Crate coherence: integrity pass, round host-initial-refactoring-r1

## State read

Round `host-initial-refactoring-r1`: the change set of the branch `host/initial-refactoring` against the initial implementation, with the `D-3` and `D-6` adjudications recorded in the register. Read in full: `.plan/00-index.md`, `.plan/host/initial-refactoring/plan.md`, `journal.md`, `decisions.md`, `README.md`; every file under `crates/host/src/`; `crates/host/tests/binary/{server,sync,support}.rs`; `bridge.py`; `setup-host.sh`; `crates/host/Dockerfile`; `.dockerignore`; `README.md`; `CONTRIBUTING.md`; `.agents/docs/{test-architecture,documentation,handoff,review-findings}.md`. `Q-1` was not reviewed, as instructed.

## Scope

**Covered**

- The protocol boundary, both ends: request line, response header, framing, size and type sets, shim timeouts, against `bridge.py`.
- The trust boundary: peer-UID admission, request and response bounds, tool reaping, absence of any clipboard write or non-image read on the request path.
- Neighbouring flows: text sync and its watcher, the lock and socket lifecycle, signal handling, the installer and Docker build inputs, `README.md` claims, `test-architecture.md` and `CONTRIBUTING.md` claims.
- Instruments: the tests and register tables that claim to hold a property, checked by mutation in scratch copies.

**Evidence gathered**

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (33 unit, 17 binary) are green at the launch commit; the Python suite passes (4 tests).
- The Docker image's command (`cargo test --locked --target x86_64-unknown-linux-musl`) passes here, and the release musl binary is 619,264 bytes, matching the journal.
- The old tree was built from the base commit into a scratch directory to compare behaviours.

**Not covered**

- The Docker image build itself (no Docker CLI; the journal records the same). Whether the default seccomp profile of a builder permits `pidfd_open` for the binary tests that run inside the build is unverified.
- The owner's end-to-end check on Fedora with a real compositor, `wl-paste` and `xsel`.
- `Q-1` (watcher death while serving).
- The correctness of the changes against their own decisions (another pass's subject), except where a finding below needs it.

## Null results

- **Protocol boundary: no drift found.** The request language, the header fields and the framing are unchanged. The shim and the host agree on the type set (`IMAGE_TYPES` in `bridge.py` equals `image::Format::ALL`), the size bound (`MAX_BYTES` equals `process::LIMIT`), and the header bound (`readline(4097)` against a header of under 100 bytes plus a short message). The `D-3` adjudication's claim that the shim cannot reach the reordered refusal holds: the shim refuses any type outside `IMAGE_TYPES` before connecting. Run over a socket, `types`, `read` and an error response produce the expected frames.
- **Trust boundary, runtime: no bypass found.** Every accepted connection goes through `admit` (peer UID, then concurrency) before a byte is read. The request is bounded by `take(4097)` under a total deadline. The payload is bounded by `LIMIT` in `capture` and by the magic check in `respond_with`. No path writes the host clipboard from a request. Every spawned tool is held by a `Tool` that kills its group when unreaped, and workers are joined by `thread::scope`. All descriptors the host creates (wake pipe, pidfds, lock, sockets) are close-on-exec.
- **Public surface: no addition.** No `pub` item beyond `pub(crate)`, no new flag or environment variable (`CLAUDE_CLIPBOARD_SOCKET` and `CLIPBOARD_STATE` predate the change), and the subcommand set is `serve` and `sync-text` only.
- Every test named in the register and in `README.md` exists in the tree.

## Findings

### integrity-1 — A bridge started with SIGHUP ignored now exits when its terminal closes

**Tier A**

- **Current behavior / contract.** `sys::wake_on_termination` installs a handler for SIGHUP unconditionally with `sigaction`. `D-7` adds SIGHUP "because a terminal that closes would otherwise kill the bridge without cleanup". The base implementation never touched SIGHUP, so a disposition inherited as ignored stayed ignored.
- **Why it is a problem.** A bridge launched under `nohup`, a `trap '' HUP` shell, or any launcher that sets SIGHUP to `SIG_IGN` used to survive the terminal that started it. `wake_on_termination` overrides that disposition, so the same launch now shuts the bridge down, and a container loses its clipboard until someone restarts it. The decision's reasoning covers a SIGHUP that would kill the process. It does not cover one that was deliberately ignored.
- **Evidence.**
  - `crates/host/src/sys.rs`, `wake_on_termination`: the loop over `[SIGTERM, SIGINT, SIGHUP]` calls `sigaction` without reading the old action (the old-action pointer is null).
  - Run on the base tree, `CLAUDE_CLIPBOARD_SOCKET=a.sock nohup <base binary> serve &`, then `kill -HUP`: the process was alive afterwards.
  - Run on the launch tree with the same command: the process was gone. `/proc/<pid>/status` then showed `SigIgn` without the SIGHUP bit.
  - `README.md` does not prescribe `nohup`; its documented installs are a terminal and niri `spawn-at-startup`. This is why the likelihood is low, but the consequence is a changed runtime behaviour, which the tier table assigns to A.
- **Required property.** A signal the bridge inherited as ignored stays ignored, or the record states that the bridge can no longer be detached from its terminal by ignoring SIGHUP. Which of the two is a decision about `D-7`, so it is routed to the architect and not answered here.

### integrity-2 — After a termination signal the bridge keeps its socket bound and silently queues new connections for up to one phase

**Tier A**

- **Current behavior / contract.** `D-7` requires that SIGTERM, SIGINT or SIGHUP "leads to shutdown as soon as the signal is delivered", and that cleanup removes the socket path and closes the listener. In `server::serve` the accept loop returns on the wake byte, then `thread::scope` joins every worker, and only after that does `Bridge` drop. So during the join the socket path exists and the listener is open but nothing calls `accept`. The base tree, by contrast, left its loop within a second and removed the path.
- **Why it is a problem.** A client that connects during the drain is accepted by the kernel into the backlog and never served: it waits for its own timeout and then sees a reset, where a stopped bridge would have refused it at once. The drain is also long. A single admitted connection that sends nothing holds it for the full request phase.
- **Evidence.**
  - `server.rs`, `serve`: `return Ok(())` from the scope closure on `ready[1].ready()`, with `bridge` dropped only after the scope returns.
  - Probe on the launch tree. One idle connection was open, then SIGTERM. One second later the socket path still existed. A new client connected, sent `{"op":"types"}`, and received nothing for 3 s. The process exited 11.9 s after the signal, and the queued client then got `ConnectionResetError`.
  - `bridge.py` gives up after 12 s per call, so a paste attempted during the window hangs for as long as the shim allows and then fails with a message that does not say the bridge is stopping.
  - Also from the same code: a second Ctrl+C is absorbed by the handler and does not hasten the exit, so an operator at the terminal sees a hung command until the phase ends or until SIGKILL. The base tree behaved the same on a second signal but never had a drain this long.
  - `README.md` says closing the terminal, Ctrl+C or SIGTERM "removes its socket" and "waits for requests in flight". It does not say that an idle connection counts as a request in flight, or that the socket stays advertised meanwhile.
- **Required property.** Once shutdown is requested, a new connection is either refused promptly or served. It is not left in a queue that nothing will drain. If the record keeps the drain as bounded by the phase deadlines, `D-7`'s "as soon as the signal is delivered" and its cleanup-order text should say which of the two it means. That is a contract call, so it is routed.

### integrity-3 — The README and the test-architecture document claim coverage of the running server that no test provides for the peer-UID check, the concurrency limit and the connection deadline's wiring

**Tier B**

- **Current behavior / contract.** `README.md` §Tests and limitations lists, under "the server as it runs", the peer UID check, the concurrency limit and "a deadline on each connection however slowly the peer sends or reads". `test-architecture.md` §The layers says the binary layer proves, among others, "deadlines". `D-7` §Implemented names `admit`'s unit tests as the witness for the refusals.
- **Why it is a problem.** The unit tests call `admit` and `Deadline` directly. Nothing runs the accept loop through them. The call to `admit`, the value of the 12 s budget and the use of the budget in `work` are the trust-boundary wiring, and none of it is pinned. A reader of the README is told the running server's admission is covered, and removing it fails nothing.
- **Evidence.** Two mutations in a scratch copy of the tree, each followed by `cargo test`:
  - In `serve`, replacing `admit(&stream, &allowed, &active)` with an unconditional `Slot` (no UID check, no concurrency limit): 33 unit and 17 binary tests passed.
  - Setting `PHASE` to 3600 s: both suites passed.
  - The binary tests all connect as the serving UID, so they cannot reach a refusal. A binary test for the concurrency limit (seventeen stalled peers, the next one refused) needs nothing provisioned.
- **Required property.** Every coverage claim in the README and in `test-architecture.md` is one a mutation of the claimed site can falsify. Either the wiring is pinned where it can be (the concurrency limit at least), or the text states that the UID check, the limit and the phase deadline are proved at the function level only.

### integrity-4 — `D-6` states the error header's bytes as an order the host does not emit

**Tier C**

- **Current behavior / contract.** `D-6`: "The header's bytes are unchanged: `{"ok":true,"size":N}` or `{"ok":false,"size":0,"error":"…"}`."
- **Why it is a problem.** `protocol::write_response` builds the header with `json!` and `serde_json` without `preserve_order`, so keys are emitted sorted. An error response is `{"error":"…","ok":false,"size":0}`, in the base tree as well as the launch tree. The two sites disagree on the spelling, although "unchanged" is true. Both ends parse JSON, and no test compares header bytes, so nothing depends on it.
- **Evidence.** A request for an unsupported operation over the socket returned `{"error":"unsupported operation","ok":false,"size":0}`. `crates/host/Cargo.toml` enables no `serde_json` feature.
- **Required property.** The register describes the header's wire form as the host emits it, or says that field order is not part of the contract.

### integrity-5 — The journal names commits, which `documentation.md` §10 forbids in a tracked file

**Tier C**

- **Current behavior / contract.** `documentation.md` §10: no tracked file uses a commit identifier as a reference or provenance.
- **Why it is a problem.** The Phase 3 entry of `.plan/host/initial-refactoring/journal.md` ends by naming the defect's span as a range of two commit identifiers. (2026-10-08: this quote reproduced them. It was replaced in place under `D-13`, and they are not carried.) The architect's 2026-10-07 entry noticed this and left it. If the branch is squashed, the range resolves to nothing.
- **Evidence.** The sentence above, found by searching the journal for hexadecimal tokens. It is the only one in the changed files.
- **Required property.** The journal names the span by what it did (the landing of the restructuring, up to the fix of step 3.4), not by hashes. The entry is append-only, so how to correct it is the owner's call.

### integrity-6 — `Deadline` states a precondition that a reachable path in `serve` does not meet

**Tier C**

- **Current behavior / contract.** `server::Deadline::new` documents that the stream "must be non-blocking", and every wait assumes it. `serve` calls `stream.set_nonblocking(true)` and, if that fails, still writes the refusal through `Deadline::new(&stream, PHASE)`.
- **Why it is a problem.** On that path the stream is blocking and carries no timeout, so the write is bounded by nothing but the size of the message. The base tree set 12 s socket timeouts on every accepted stream. The failure is not realistic for a fresh accepted socket, and the refusal is under 100 bytes, which is why this is C. The architect's entry already records the same observation as noticed and not acted on.
- **Evidence.** `server.rs`, the `admitted` binding and the `Err(error)` arm that follows it, against the doc comment on `Deadline::new`.
- **Required property.** A path that hands `Deadline` a stream does so with its precondition met, or the precondition is stated at the call as something the code relies on and cannot check (`documentation.md` §5.1).

## Tier counts

A: 2 (`integrity-1`, `integrity-2`). B: 1 (`integrity-3`). C: 3 (`integrity-4`, `integrity-5`, `integrity-6`).
