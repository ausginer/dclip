# Record register

The one register of the record. Every decision (`D-`), finding (`F-`), question (`Q-`) and investigation (`I-`) is a `####` entry here, opening with its identifier; one entry reads back standalone with

```sh
awk -v re="^#### D-1( —|$)" '$0 ~ re {f=1;print;next} f && /^#{1,4} /{exit} f' .plan/00-index.md
```

Entries are append-only and dated. A substantive amendment to a decision mints a new decision that supersedes it. Supporting documents live beside this file, grouped by the work that produced them, and name entries without claiming them.

| Work                                                   | Documents                                                                                     |
| ------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| Host initial refactoring, opened 2026-10-06            | [`host/initial-refactoring/`](host/initial-refactoring/README.md) — analysis, alternatives, plan, journal |
| Review round `host-initial-refactoring-r1`, 2026-10-07 | [`reviews/host-initial-refactoring-r1/`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) — four pass reports and their consolidation |
| Review round `host-initial-refactoring-r2`, 2026-10-09 | [`reviews/host-initial-refactoring-r2/`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) — four pass reports and their consolidation |
| `lexopt` and installable packages, opened 2026-10-09    | [`packaging/`](packaging/README.md) — alternatives, plan, journal                              |

---

## Decisions

#### D-1 — The host crate is split into modules by responsibility, and `unsafe` lives in one of them

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-7` · Alternatives in [`decisions.md`](host/initial-refactoring/decisions.md)

`crates/host/src/main.rs` carries the protocol policy, child-process management, the server lifecycle, text sync and nine `unsafe` blocks in one 572-line file. The responsibilities change for different reasons and are tested by different layers, so each gets a module, and every FFI call moves behind a safe function in one module that is the only place the compiler permits `unsafe`.

The intended modules — names are illustration, the responsibilities are the requirement: `main` (dispatch and exit status), `cli` (argument parsing), `image` (supported formats and magic bytes), `protocol` (request parsing, response policy, framing), `clipboard` (what is asked of `wl-paste` and `xsel`), `process` (running a tool under a deadline), `server` (socket, lock, accept loop, connections), `sync` (text sync), `sys` (every libc call).

Required properties:

- Each module owns one responsibility from the list above; no module reaches into another's internals, and nothing is made `pub` beyond `pub(crate)` for a test.
- `unsafe` is denied crate-wide by lint and allowed in exactly one module. Every `unsafe` block carries a `// SAFETY:` comment, enforced by `clippy::undocumented_unsafe_blocks` at deny level.
- `unwrap` is denied outside tests by `clippy::unwrap_used`, so `CONTRIBUTING.md` §Errors' rule has a witness.
- Lint levels are declared once, in the workspace manifest's `[lints]` table, and the crate inherits them.
- `rust-version` is raised to the lowest release that provides every `std` API the refactoring uses in place of `libc`, at least 1.89 for `File::try_lock`. `clippy::incompatible_msrv` is the witness that the figure is honest.
- The `flock` call is replaced by `std::fs::File::try_lock`; `peer_uid` stays on `libc` because `UnixStream::peer_cred` is unstable (`I-2`).
- The restructuring commit changes no behaviour: every test that passed before it passes after it unchanged, apart from test-module moves.

##### D-1 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

Modules `cli`, `clipboard`, `image`, `process`, `protocol`, `server`, `sync` and `sys` under `crates/host/src/`. `sys.rs` alone carries `#![allow(unsafe_code)]`; the workspace `[lints]` table denies `unsafe_code`, `clippy::undocumented_unsafe_blocks` and `clippy::unwrap_used`, and `clippy.toml` allows `unwrap` in tests. `rust-version` is 1.89. The lock is `File::try_lock` in `server::serve`; `sys::peer_uid` stays on `libc`.

2026-10-07 · The restructuring commit carried one behaviour change, the order of `D-3`'s request refusal against the listing. It is accepted in `D-3` §Adjudicated.

#### D-2 — Unit tests live in sibling files, and the binary gains a black-box test layer

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-7`, `F-9` · Amends `CONTRIBUTING.md` §Tests and `.agents/docs/test-architecture.md` §The layers

Two changes to where tests live.

**Unit and OS-boundary tests move out of the source file, not out of the module.** A module `foo.rs` declares `#[cfg(test)] mod tests;` and its tests live in `foo/tests.rs`. They keep crate-private access, so no seam is published for them, and the production file reads without them.

**A binary-level layer is added under `crates/host/tests/`.** It runs the built `claude-clipboard-host` through its real surface — arguments, exit status, stderr, the socket, signals — with stand-in `wl-paste` and `xsel` scripts placed first on the child's `PATH`. The stand-ins replace the Wayland session, not the bridge's own lifecycle: what these tests prove is the bridge's socket, lock, signal, deadline and process handling, which no unit test can reach and which today has no test at all (`F-9`). This is consistent with the criterion `test-architecture.md` already states — a test is placed by what it must reach — because the binary's command line and socket are a surface the test reaches without anything being made public.

`CONTRIBUTING.md` §Tests currently reads: _"A Rust test module is `#[cfg(test)] mod tests` beside the code it tests. The host is one binary crate with no public surface, so there is no `tests/` layer"_. It is amended, in the change that introduces the layer, to state both rules above; the old wording goes into its change record and is preserved here.

Required properties:

- Every production module with tests declares `#[cfg(test)] mod tests;` and keeps them in `<module>/tests.rs`. No `#[test]` function remains in a production file.
- Binary tests drive only the built binary, located with `env!("CARGO_BIN_EXE_claude-clipboard-host")`. They add no `pub` item, no environment variable, flag or subcommand to the binary (`CONTRIBUTING.md` §4: a test seam is not a reason to publish a value).
- They need nothing provisioned beyond `/bin/sh` and coreutils, so they run in the default `cargo test` on a fresh checkout and in the Docker build. They take no dev-dependency unless a later decision accepts one.
- Each test uses its own directory, removes it when done, and keeps its socket path within the 107-byte `sun_path` limit whatever the checkout's location.
- A test that starts `serve` terminates it on every path, including a failed assertion.
- `crates/host/Dockerfile` and `.dockerignore` admit `crates/host/tests`, so the image build runs the layer before exporting the binary.
- `test-architecture.md` §The layers and §Failure interpretation describe the new layer; `README.md` §Tests and limitations states what it covers.

##### D-2 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

Sibling `tests.rs` files beside `image`, `process`, `protocol`, `server`, `sync` and `sys`. The binary layer is one target, `crates/host/tests/binary/`, with `support.rs` and modules `cli`, `protocol`, `server` and `sync`. `Dockerfile` and `.dockerignore` admit it; the image build was not run, because no Docker CLI was available (journal). `CONTRIBUTING.md` §Tests and `test-architecture.md` carry the amendment and their change records; `README.md` §Tests and limitations states the coverage.

#### D-3 — Domain values are types: image formats, requests, clipboard queries and commands

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-7` · Measured in `I-3`

The host passes MIME types, operations, `wl-paste` argument arrays and subcommands around as strings. Each becomes a closed type, parsed once at the boundary where the string arrives.

- **An image format is an enum** with one variant per accepted MIME type: PNG, JPEG, `image/jpg`, GIF and WebP. The variant knows its MIME string and its magic-byte check. JPEG and `image/jpg` are separate variants with one check, because the host requests from `wl-paste` the exact string the clipboard offered.
- **A request is an enum**: types, or read with an optional wanted type. It is built by hand from `serde_json::Value`, not by `serde` derive: derive costs 28 KiB of release binary and 2.4 s of clean build, and its `deny_unknown_fields` does not refuse an unknown field beside a unit variant of an internally tagged enum — the case the existing `should_reject_invalid_requests` pins (`I-3`).
- **The clipboard seam is a closure over a query enum**: list types, read an image of a format, read text. It replaces the closure over `wl-paste` argument arrays, so the policy and its tests no longer spell `wl-paste` flags.
- **A command is an enum** produced by the argument parser, with the `serve` options as a struct.
- **Errors stay boxed messages**, as `CONTRIBUTING.md` §Errors requires. No error enum and no `thiserror`: nothing branches on the classification.

Required properties:

- The accepted request language is unchanged: a JSON object of at most 4096 bytes including its newline, keys within `op` and `type`, `op` either `types` or `read`; for `read`, `type` absent, `null`, `"image"` or an offered supported MIME type; for `types`, any `type` value is ignored. Every request the existing tests refuse is still refused.
- The preference order PNG, JPEG, `image/jpg`, GIF, WebP is the enum's declaration order, and it alone decides both the `types` response order and which image a read without a type returns. Each offered type appears at most once in the response.
- A match over the format enum lists its variants instead of using a `_` arm.
- The response policy remains testable without Wayland through the query closure (`test-architecture.md` §The layers).
- User-facing error text is unchanged except where another decision changes it.

##### D-3 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`image::Format` (with `ALL` in declaration order, pinned by `should_list_formats_in_declaration_order`), `protocol::Request::parse`, `clipboard::Query` as the seam of `protocol::respond_with` and `sync::sync_text`, `cli::Command` and `cli::ServeOptions`. A `read` naming an unsupported `type` is now refused before the clipboard is listed; the message is unchanged.

##### D-3 §Adjudicated

2026-10-07 · Architect · The error-precedence change conforms. No superseding decision, no remediation.

**What changed.** In the initial implementation, `respond_with` listed the clipboard before it looked at `type`. `protocol::Request::parse` now refuses a `read` whose `type` is an unsupported string or not a string, and it does so before any listing. Two observable outcomes differ, both for that request alone:

- When the listing would also have failed, the peer now gets `requested image type is not in host clipboard`. It used to get the listing's failure: `wl-paste timed out`, `wl-paste failed; check it on Fedora`, or a spawn error.
- `wl-paste` is no longer run.

Every other request, under every clipboard state, gets the outcome it got before.

**Why it conforms.**

- **The statement above entails it.** Each value is "parsed once at the boundary where the string arrives", and the read variant carries a `Format`. There are two ways to keep the old precedence. A request could hold an unsupported type, which is the unrepresentable state this decision exists to remove. Or the parser could be split around the listing. Either way, a `wl-paste` spawn is spent on a request that cannot succeed (`CONTRIBUTING.md` §0).
- **No required property is broken.** The accepted language is unchanged, every refused request is still refused, and no message string changed. "User-facing error text is unchanged" governs the wording of each message. It does not govern which of two independent failures is reported. Nothing fixed that precedence: not these properties, not [`analysis.md`](host/initial-refactoring/analysis.md) §Properties worth keeping, and not a test. By `CONTRIBUTING.md` §13 it lies outside the contract. Reading the property this way changes nothing the decision requires of the code, so it is not an amendment (`documentation.md` §6).
- **The shim cannot reach the changed case.** `bridge.py` refuses locally any type outside `IMAGE_TYPES`, which is the set `Format` accepts. Only a peer other than the shim can send such a request, and it gets `ok:false` either way.

**The one tension is with `D-1`.** `D-1`'s last required property is that the restructuring commit changes no behaviour, and this ordering change landed in that commit. The property's witness held: every test passed unchanged apart from its mock. The journal also recorded the difference when it was made. The deviation is accepted as recorded. No test is owed, because pinning the precedence would turn it into contract.

#### D-4 — A tool is read event-driven, under one total deadline, by a guard that always reaps

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-1`, `F-7` · Measured in `I-1`

`capture` reads at most 16 KiB and then sleeps 1 ms, so throughput is capped near 14 MiB/s and a 63 MiB image misses its 4 s bound (`F-1`). The wait becomes event-driven: the capturing thread blocks until the tool's stdout is readable, the tool has exited, or the deadline has passed, whichever comes first, using `poll` over the stdout pipe and a pidfd for the child. Child exit is observed through the pidfd rather than by polling `try_wait`, so no fixed-interval wait remains anywhere in the capture.

The guard around the child stops being a `Child` with a `bool` (`CONTRIBUTING.md` §Types: an enum over a `bool`). Its type distinguishes a child that has been reaped from one that has not.

Required properties:

- No fixed-interval sleep or timed poll tick on the capture path. Time spent in a capture is the tool's own time plus the cost of moving its bytes.
- One deadline bounds the whole capture — reading, writing stdin and waiting for exit — and is passed as a parameter, so tests use millisecond bounds and the production values stay where they are now: 4 s for `wl-paste` and `xsel -ib`, 1 s for `xsel -ob`.
- Bytes are read into the buffer that is returned, without an intermediate chunk copy. The 64 MiB limit is enforced before the buffer grows past it.
- A child that has been reaped is never signalled: its PID or process group may already belong to someone else. A child that has not been reaped is killed by process group and reaped on every exit path, panics included.
- On success the tool's process group is left alone, so the daemon `xsel -i` forks to own the X11 selection survives. Each tool runs in its own process group, so that killing the watcher's group at shutdown never takes the selection owner with it.
- The stdin writer cannot outlive the capture: it is joined after the child is killed, never before, because joining first blocks on a tool that stopped reading.
- A binary-level test delivers a 64 MiB image (the limit) through the socket from a stand-in `wl-paste` and asserts it arrives byte-exact, under the production deadline.
- Re-measured as `I-1` measured it, on the same machine: a 63 MiB capture completes in well under its 4 s bound, and the figure is recorded in the journal.

##### D-4 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`process::capture` waits in `sys::poll` on the stdout pipe and a `sys::pidfd_open` descriptor under one deadline, reading with `read_to_end` through `take(LIMIT + 1 - len)` into the returned buffer. `process::Tool` wraps `State::{Unreaped, Reaped}`; `Drop` lives on the wrapper so that the transition to `Reaped` never signals. Tests: `should_not_signal_the_group_of_a_reaped_child`, `should_leave_the_group_of_a_successful_tool_alone`, `should_refuse_output_over_the_limit`, and the binary `should_deliver_an_image_at_the_64_mib_limit_byte_exact`. Re-measured: 63 MiB in about 38 ms (journal).

##### D-4 §Adjudicated

2026-10-08 · Architect · `F-13` is a defect against this decision, not a conflict within it. D-4 stands. No superseding decision is needed, but remediation is owed.

**Ruling.** `F-13` sets two of the properties above against each other: one deadline bounds the whole capture, stdin included, and a reaped child is never signalled. They conflict only for an implementation that writes stdin from a thread the capture must join.

- Once the tool is reaped, nothing may signal its group. Such a thread therefore waits for whoever still holds the pipe, past the deadline: 3.01 s against 200 ms in `F-13`'s probe.
- A stdin write that waits under the capture's own deadline meets both properties. When the capture ends, on any path, the bridge closes its end of the pipe, and nothing is left to join.
- In this design that write is non-blocking, inside the capture's `poll` loop, and the writer thread is removed. "The stdin writer cannot outlive the capture" is then true by construction.

**Excluded:**

- **Signalling the group after the leader has been reaped.** Once the group's last member exits, its ID may name another group.
- **Detaching the writer thread.** It would outlive the capture.

**A process the tool left in its group is left alone**, even if it holds the pipe after the tool failed. That matches the success path: the guarantee covers the tool the bridge spawned, not the tool's descendants once the tool has been reaped.

**Owed:**

- A unit test with `F-13`'s shape: the tool exits non-zero while a member of its group holds stdin open and does not read it. The capture returns its failure within a millisecond deadline.
- The existing timeout, reaping and stdin tests still pass.

##### D-4 §Remediated

2026-10-09 · Branch `host/initial-refactoring`. What §Adjudicated owed.

`process::capture` makes the tool's stdin non-blocking and writes it from its own `poll` loop, which watches stdin for writability beside stdout and the pidfd, through `sys::Poll::optional`. The pipe closes once the input is written, once the tool stops taking it, or when `capture` returns, so there is no writer thread and nothing to join. A failed write is reported only if the tool exits successfully, as before. Tests: `should_fail_within_the_deadline_while_a_group_member_holds_unread_input`, which took 3.0 s against its 200 ms deadline before the change; `should_feed_input_larger_than_a_pipe_while_reading_the_output`; and the existing timeout, reaping and stdin tests, unchanged.

#### D-5 — The clipboard's type listing is parsed as bytes

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-4`

`wl-paste --list-types` returns whatever MIME strings the source application offered, and one non-UTF-8 entry currently fails the whole request (`F-4`). Supported types are ASCII, so the listing is split and matched as bytes, and a line that is not a supported type is skipped whatever it contains.

Required properties:

- No UTF-8 validation of a listing as a whole, in the response policy or in text sync.
- A line equal to a supported MIME type selects that format. Any other line, including one that is not valid UTF-8, is ignored without error.
- Text sync's rule is unchanged: it proceeds only when no line starts with `image/` and some line starts with `text/plain`, and both tests are byte prefixes.
- A unit test pins a listing that mixes a supported type with a non-UTF-8 line.

##### D-5 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`clipboard::lines` splits a listing as bytes; `protocol::offered` and `sync::may_sync_text` compare bytes. Tests: `should_serve_an_image_when_another_listing_line_is_not_utf8`, `should_sync_plain_text_when_another_listing_line_is_not_utf8`.

#### D-6 — Each connection phase has a total deadline, and the header is framed in one buffer

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-2`, `F-5`

Socket timeouts in `std` bound a single syscall, so a peer that sends one byte at a time holds a worker for as long as it likes, and sixteen such peers lock everyone else out (`F-2`). The request read and the response write each get a total deadline measured from the start of the phase, and every blocking call is given the time that remains.

`serde_json::to_writer` writes the header straight into the unbuffered socket, one syscall per JSON token (`F-5`). The header is serialised into its own small buffer and sent with the payload, either as one vectored write or as two writes, without copying the payload.

Required properties:

- Reading the request line completes or fails within 12 s of accepting the connection, whatever the peer's pacing. Writing the response completes or fails within 12 s of starting it. A worker holds its slot for at most these bounds plus the tools' own deadlines.
- A peer that misses a deadline is dropped, and its slot is released.
- The bounds are parameters of the functions that enforce them, so a unit test proves each one with a millisecond bound, and the production values remain 12 s.
- A response is the header, a newline and the payload, written in a number of syscalls that does not depend on the header's content. The payload is never copied to join it to the header.
- The header's bytes are unchanged: `{"ok":true,"size":N}` or `{"error":"…","ok":false,"size":0}` followed by a newline.

> 2026-10-08 — Corrected in place, as a non-substantive edit (`F-16`). The error header was spelled `{"ok":false,"size":0,"error":"…"}`. The host has always sent it with its keys sorted, because `serde_json` is built without `preserve_order`, and "unchanged" was always the requirement. What the code owes is the same as before.

##### D-6 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`server::Deadline` makes the stream non-blocking and waits in `sys::poll` for the time remaining; per-call socket timeouts were not enough, because the kernel re-arms `SO_SNDTIMEO` within one large write. `protocol::write_response` serialises the header into its own buffer and sends it with the payload by `write_vectored`. Tests: `should_frame_any_response_in_one_write_to_a_writer_that_takes_it_all`, `should_fail_a_trickled_request_within_its_deadline`, `should_fail_a_slowly_read_response_within_its_deadline`.

##### D-6 §Adjudicated

2026-10-07 · Architect · The mechanism change conforms. No superseding decision, no remediation.

**What changed.** The argument above implies a mechanism: set each socket call's timeout to the time that remains. The implementation uses a different one. `server::Deadline` makes the accepted stream non-blocking, and every wait is a `sys::poll` given the time that remains. That still satisfies the argument's own sentence, because the one call that blocks is now `poll`, and it is the call given the remaining time. Every required property holds, and the tests named in §Implemented pass on the branch as of this date. The required properties are the whole of what this decision requires of the code (`documentation.md` §6). A change of mechanism alters none of them, so it is not a substantive amendment.

**The argument's premise was wrong, and this note corrects it.** A socket timeout does not bound even one call. Measured 2026-10-07 on the repository's devcontainer, kernel 7.2.7, with a scratch Python probe that was not retained:

- Setup: an `AF_UNIX` stream with `SO_SNDTIMEO` at 200 ms, and a peer draining 64 KiB every 50 ms.
- One blocking `send` of 8 MiB returned after 6.30 s, having sent every byte.

The kernel takes the timeout afresh for each buffer it allocates within the call. Resetting the timeout before each call would therefore have failed the first required property against a slow reader. That is the failure the implementer's first attempt showed. The argument's text above is left as written, and this note carries the correction.

**Not the rejected event loop.** The rejected alternative in [`decisions.md`](host/initial-refactoring/decisions.md) §Connection deadlines and framing is one `poll` loop serving every connection. Here each worker still owns its connection on its own thread and polls only its own stream, so `CONTRIBUTING.md` §Language and platform's thread per connection stands.

#### D-7 — The server wakes on events, joins its workers, and survives a failed connection

2026-10-06 · Accepted · Implemented 2026-10-06 · **Superseded 2026-10-08 by `D-11`** · Addresses `F-6` · Mechanism chosen by `I-2`

Shutdown currently relies on a flag checked after a 1 s `poll` timeout, because a signal may be delivered to a worker thread (`F-6`). The signal handler instead writes one byte to a non-blocking pipe, and the accept loop waits on the listener and the pipe with no timeout. `signalfd` was the alternative and is rejected: it needs SIGTERM and SIGINT blocked in every thread, and `std::process::Command` passes the blocked mask to every tool it spawns (`I-2`), so `wl-paste`, `xsel` and every sync process would then ignore SIGTERM.

Workers run inside `std::thread::scope`, which joins every one of them before `serve` returns, so a tool started for a connection is reaped by the bridge rather than abandoned at exit. Every worker is bounded by `D-4` and `D-6`, so shutdown is bounded too.

Required properties:

- No timed wake-up in the accept loop. SIGTERM, SIGINT or SIGHUP leads to shutdown as soon as the signal is delivered. SIGHUP joins the handled set because a terminal that closes would otherwise kill the bridge without cleanup.
- A signal handler does only async-signal-safe work: it writes to the wake pipe and nothing else.
- `serve` returns only after every worker has finished; no tool spawned for a connection outlives the process.
- A failure that concerns one connection never ends the server. That covers reading its peer credentials, setting its timeouts, a refused UID, the concurrency limit and a failure to start its worker thread. Each gets an error response where one can still be written. An `accept` error from a connection that vanished, or `EINTR`, is retried; any other `accept` error ends `serve` with that error.
- Cleanup order is a property of the types: the watcher stops first, then the socket path is removed, then the listener closes, then the lock is released last, because an instance that unlocks before removing its path can delete the socket a successor has just bound. The order is stated where it is enforced.
- The concurrency limit stays at 16. Each refusal message is unchanged.

##### D-7 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`sys::wake_on_termination` installs SIGTERM, SIGINT and SIGHUP handlers with `SA_RESTART` that write to a leaked, non-blocking pipe end; the accept loop in `server::serve` polls the listener and the pipe with no timeout, inside `thread::scope`, starting workers with `Builder::spawn_scoped`. `server::Bridge` states the cleanup order by field order. `server::admit` refuses per connection. Tests: binary `should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal` and `should_remove_the_socket_and_exit_0_on_a_termination_signal` (now with SIGHUP); unit `should_refuse_a_peer_whose_uid_is_not_allowed`, `should_refuse_a_connection_while_every_place_is_taken`, `should_release_a_place_when_its_connection_is_done`.

#### D-8 — The text-sync watcher cannot outlive the bridge

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-3`

`wl-paste --watch` is killed only when the bridge exits through its destructors, so a bridge killed by SIGKILL or the OOM killer leaves the watcher running under init (`F-3`). The watcher is started with the parent-death signal set to SIGKILL, in the child before `exec`.

Required properties:

- However the serving process ends, the watcher process ends with it. A binary-level test kills `serve` with SIGKILL and asserts that the stand-in watcher is gone.
- The race in which the parent dies between `fork` and the `prctl` call is closed: the child checks its parent after setting the signal and exits if the parent is already gone.
- The watcher is spawned from the thread that lives for the whole of `serve`, because the parent-death signal follows the thread that forked, not the process.
- A sync process already running when the watcher dies runs to completion within its own tool deadlines. It is not tracked further.

##### D-8 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`sys::kill_with_parent` sets `PR_SET_PDEATHSIG` to SIGKILL in `pre_exec` and fails the spawn if the parent has already changed; `clipboard::watch` uses it, called from `server::serve` on the main thread. Test: binary `should_end_the_watcher_when_serve_is_killed`.

##### D-8 §Adjudicated

2026-10-08 · Architect · `F-17`: the last property above is scoped to the death of the serving process. No superseding decision is needed. The code is unchanged, and only its comment is owed.

**Ruling.** The property says that a sync already running when the watcher dies runs to completion. It belongs to this decision's subject, the parent-death signal. It says what `PR_SET_PDEATHSIG` does not reach: the signal kills the watcher alone, so a running sync is not tracked further. It is not a guarantee about a graceful shutdown. That path is governed by `D-4`, which requires an unreaped child to be killed by process group. `D-4` also names "killing the watcher's group at shutdown" as intended. Read any wider, the property would contradict `D-4` and require nothing that `D-8` was argued for. This reading changes nothing the code was built to, so it is not an amendment (`documentation.md` §6).

**What happens to an in-flight sync**, on each way the bridge ends:

- **SIGTERM, SIGINT or SIGHUP.** The bridge kills the watcher's process group, and every `sync-text` the watcher started dies with it. A sync killed while it is still writing more than a pipe buffer of text to `xsel -ib` may leave X11 a truncated selection. That was reasoned from the code and not reproduced. The next copy replaces it. This is accepted: stopping the bridge stops text sync, which is what `README.md` says.
- **SIGKILL, the OOM killer, or any other death of the serving process.** Only the watcher is signalled. A running sync finishes within its own tool deadlines.

**Owed:** the doc comment on `clipboard::watch` states both cases. It currently states only the second, and states it without condition.

##### D-8 §Remediated

2026-10-09 · Branch `host/initial-refactoring`. The doc comment on `clipboard::watch` states both cases §Adjudicated lists: a graceful shutdown kills the watcher's group and every sync in it, and parent death kills the watcher alone. No code changed.

#### D-9 — The `handle-stdio` subcommand is deleted

2026-10-06 · Accepted · Implemented 2026-10-06 · **Superseded 2026-10-08 by `D-12`** · Addresses `F-8`

`handle-stdio` answers one request from stdin with no peer check. It is undocumented, absent from the usage line, and has no caller in the repository, `setup-host.sh` or the shim. Nothing is released (`CONTRIBUTING.md` §8), so it is deleted rather than documented. The binary-level tests of `D-2` reach the protocol through `serve`, which is the published surface.

Required properties:

- The binary accepts `serve [--sync-text] [--allow-uid UID]…` and `sync-text`, and nothing else.
- Any other invocation prints the usage line and exits with status 1, and a binary-level test pins that.

##### D-9 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

Deleted from `cli::parse` and `main`. Test: binary `should_refuse_handle_stdio_as_an_unpublished_invocation`.

#### D-10 — Text sync's decision sequence is testable without Wayland or X11, and unchanged

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-9`

Only the type filter of text sync has a test. The sequence around it has none, and the equality check in that sequence is what stops sync feeding back into itself through XWayland. The sequence is: honour `CLIPBOARD_STATE`, list the types, filter them, read the text, compare with X11, list and filter again, write. It moves behind the same kind of seam the response policy has: the tool calls are passed in.

Required properties:

- The sequence, its order and its outcomes are unchanged.
  - `CLIPBOARD_STATE` other than `data` or `sensitive` skips sync.
  - Unset, it proceeds.
  - X11 text equal to the Wayland text skips the write.
  - A failed X11 read is treated as "different".
  - The second listing is taken after the X11 read and before the write.
  - The watcher's stdin is drained before anything else.
- The seam is a closure or generic parameter, never a trait object or a trait with one implementor (`CONTRIBUTING.md` §10).
- Unit tests pin each of the following:
  - the skip on unchanged X11 text;
  - the skip when an image appears by the second listing;
  - the state filter;
  - that the write carries exactly the text that was read.

##### D-10 §Implemented

2026-10-06 · Branch `host/initial-refactoring`.

`sync::sync_text(state, wayland, x11_text, set_x11_text)`, with `sync::run` draining stdin and reading `CLIPBOARD_STATE`. Tests in `sync/tests.rs`: `should_write_exactly_the_text_that_was_read`, `should_skip_the_write_when_x11_already_holds_the_text`, `should_treat_a_failed_x11_read_as_different_text`, `should_skip_the_write_when_an_image_appears_by_the_second_listing`, `should_sync_only_in_the_data_and_sensitive_states`; binary `should_write_offered_plain_text_to_x11`.

#### D-11 — The server keeps inherited ignored signals, stops advertising its socket once shutdown is requested, and drains only work in progress

2026-10-08 · Accepted · Implemented 2026-10-09 · **Superseded 2026-10-09 by `D-15`** · Supersedes `D-7` · Addresses `F-6`, `F-10`, `F-11`, `F-21` · Alternatives in [`decisions.md`](host/initial-refactoring/decisions.md)

`D-7`'s mechanism stands:

- a wake pipe written by the signal handlers;
- an accept loop with no timeout;
- workers inside `std::thread::scope`;
- refusals per connection;
- a cleanup order carried by types.

Round `host-initial-refactoring-r1` found three places where `D-7` asked for the wrong thing. This decision restates `D-7` with them corrected, so that the lifecycle contract can be read from one entry.

**Inherited dispositions (`F-10`).** `D-7` installed handlers for SIGTERM, SIGINT and SIGHUP whatever their disposition. Its reason for SIGHUP was a closing terminal, which would otherwise kill the bridge without cleanup. A disposition inherited as ignored means the launcher has already decided that the bridge survives that signal. `nohup` does this for SIGHUP, and a shell starting a background job without job control does it for SIGINT. Overriding that choice is nannying the operator (`CONTRIBUTING.md` §1.1), and the cost was measured: a bridge started under `nohup` now dies with its terminal. The rule is the same for all three signals, because the reasoning does not depend on which signal it is.

**When shutdown takes effect (`F-11`).** `D-7` dropped the socket only after every worker had been joined. During the drain the path stayed bound, the kernel queued connections that nothing would accept, and a peer that had sent nothing held the drain for a whole request phase. Measured with one idle connection: 11.9 s from SIGTERM to exit, with a queued client left unanswered and then reset. "As soon as the signal is delivered" now means two things. The bridge stops advertising itself at once. The drain then waits only for requests that have already been received, because only they have work in progress. A peer still sending its request line has started nothing, so it is told the bridge is stopping.

**`EINTR` (`F-21`).** The listener is non-blocking, and a non-blocking `accept` never sleeps, so a signal cannot fail it with `EINTR`. Probed: no `EINTR` in 200,000 non-blocking `accept` calls across 7.4 million handler runs, against `EINTR` on every one of 2,000 blocking calls. The premise behind `D-7`'s `EINTR` clause, and behind `SA_RESTART`, never held. Every blocking call left in the serving process goes through a `std` path that retries `EINTR` itself. `poll` is never restarted, whatever the flag says, so its callers retry it.

Required properties:

- **No timed wake-up in the accept loop.** Shutdown begins at the loop's next wake after a handled signal.
- **Inherited ignored signals stay ignored.** The handled signals are SIGTERM, SIGINT and SIGHUP, each only if its disposition is not ignored when `serve` starts. A signal inherited as ignored stays ignored, and the bridge does not stop for it.
- **The handler only writes to the wake pipe.** It does async-signal-safe work and nothing else.
- **The bridge stops advertising before any worker is joined.** Once shutdown is requested, the watcher is stopped, then the socket path is removed, then the listener is closed. A connection that the kernel queued but `serve` never accepted is reset when the listener closes. A later connect fails at once.
- **A peer still sending its request is turned away.** If a connection's request line has not fully arrived when shutdown is requested, it gets an error response, `bridge is shutting down`, and is closed without waiting further. A connection whose request has arrived is answered, with its tools and its response write each within their own bounds (`D-4`, `D-6`).
- **`serve` returns only after every worker has finished.** No tool spawned for a connection outlives the process. The lock is released last, after every worker has finished. A successor started during the drain fails on the lock, exactly as it would against a running bridge.
- **A failure that concerns one connection never ends the server.** That covers:
  - reading its peer credentials;
  - making its stream non-blocking;
  - a refused UID;
  - the concurrency limit;
  - a failure to start its worker thread.

  Each gets an error response where one can still be written.
- **Which errors are retried:**
  - `poll`: `Interrupted` is retried.
  - `accept`: `WouldBlock` is retried (the readiness was taken by a connection that vanished), and so is `ConnectionAborted`. Any other `accept` error ends `serve` with that error.
  - Nothing else in the signal or accept path answers `EINTR`. There is no `Interrupted` arm on `accept`, and the handlers are installed without `SA_RESTART`.
- **The cleanup order is a property of the types**, and it is stated where it is enforced.
- **The concurrency limit stays at 16**, and each existing refusal message is unchanged.
- **Witnesses, at the binary level:**
  - `serve`, started with SIGHUP ignored, survives SIGHUP and still answers a request, then stops on SIGTERM.
  - With an idle connection open, after SIGTERM the socket path is gone. The idle peer receives `bridge is shutting down`, and `serve` exits 0 well inside one request phase.
  - `should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal` still passes.
  - The concurrency test that `F-14` §Ruling requires.
##### D-11 §Implemented

2026-10-09 · Branch `host/initial-refactoring`.

- **Dispositions.** `sys::wake_on_termination` reads each signal's current action and installs the wake handler only where it is not `SIG_IGN`. The handlers are installed without `SA_RESTART`.
- **Stopping advertising.** `server::Bridge` holds the watcher and the `Socket` only, in that drop order. It is moved into the `thread::scope` closure, so it drops as the accept loop ends, before the scope joins any worker. The lock is a separate local, dropped explicitly after the scope returns.
- **The request phase at shutdown.** `server::work` reads the request through `Deadline::or_until_shutdown`, which also waits on the wake pipe and ends a wait with `bridge is shutting down`. Bytes already sent are still read. The response phase has no stop.
- **Retries.** `accept` retries `WouldBlock` and `ConnectionAborted`, and nothing else. Every `poll` caller retries `Interrupted`.

Tests:

- binary `should_keep_a_signal_ignored_when_serve_inherits_it_ignored`, `should_unadvertise_at_once_and_turn_away_an_idle_peer_on_a_signal`, `should_refuse_a_connection_over_the_limit_and_keep_serving`, `should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal` and `should_remove_the_socket_and_exit_0_on_a_termination_signal`;
- unit `should_turn_away_a_request_still_arriving_at_shutdown` and `should_read_a_request_that_arrived_before_shutdown`.

The arms `F-14` §Ruling waived have no test: a failure to read peer credentials, a refused UID, a failure to make the stream non-blocking, and a failure to start a worker thread.

#### D-12 — Only the published invocations run, and a refused invocation is told what was wrong

2026-10-08 · Accepted · Implemented 2026-10-09 · Supersedes `D-9` · Addresses `F-8`, `F-18`, `F-24` · Alternatives in [`decisions.md`](host/initial-refactoring/decisions.md)

`D-9` deleted `handle-stdio`, and that stands. Its second property, "any other invocation prints the usage line", contradicted `D-3`'s unchanged error text for a `serve` whose options are malformed. The implementation followed `D-3`, and `D-3` gives the better outcome (`CONTRIBUTING.md` §12). A user who chose the right subcommand and got one argument wrong is better served by a line naming that argument than by the usage line. One of those messages names nothing, though: `serve --allow-uid alice` prints `invalid digit found in string`, which is `std`'s parse error. The rule below makes it name the problem.

`D-9`'s witness was a test of its own, and it repeats the usage-refusal test (`F-24`). The word `handle-stdio` is worth pinning, because it is the shape that was deleted. It needs a row in that table, not a second test.

Required properties:

- The binary accepts `serve [--sync-text] [--allow-uid UID]…` and `sync-text`, and nothing else. `handle-stdio` is not accepted.
- An invocation that names no published subcommand, or that gives `sync-text` any argument, prints the usage line and exits 1.
- A `serve` with an argument it does not accept exits 1 with one line naming the argument at fault:
  - an unknown option is named: `unknown argument: --bogus`, unchanged;
  - an `--allow-uid` with nothing after it says so: `missing UID`, unchanged;
  - an `--allow-uid` value that is not a number names that value and says a UID was expected. This replaces `invalid digit found in string`, and the exact wording is the implementer's.
- **Witnesses:**
  - One binary table test of usage refusals, with `handle-stdio` among its rows and no setup that its assertion does not use.
  - One binary table test of malformed `serve` arguments, asserting exit status 1 and each message.
  - Both end on every outcome, including one where `serve` starts (`F-15`).
##### D-12 §Implemented

2026-10-09 · Branch `host/initial-refactoring`.

`cli::parse` reports an `--allow-uid` value that does not parse as `not a UID: <value>`. Tests: binary `should_print_usage_and_exit_1_for_an_unpublished_invocation`, whose table has a `handle-stdio` row, and `should_exit_1_naming_the_argument_at_fault_for_malformed_serve_arguments`, which asserts each message exactly. Both run the binary through `Scratch::run`, which kills it and fails the test if it has not exited within 10 s.

#### D-13 — A commit identifier found in a tracked file is replaced where it stands, and the record does not carry it

2026-10-08 · Accepted · Implemented 2026-10-08 · Addresses `F-19` · Amends `.agents/docs/documentation.md` §6 and §10

Two rules of `documentation.md` collide when a commit identifier is already in the record:

- §10 forbids a commit identifier in any tracked file.
- §6 makes the record append-only, and carries a withdrawn text in a dated note.

Leaving the identifier keeps the violation. Withdrawing it under §6 copies the identifier into the note, which keeps the violation too. §6 exists so that the record never destroys the only copy of something a reader needs. A commit identifier is not such a thing: §10's own reason is that it "carries nothing a reader can check", and on a squashed branch it resolves to nothing. So §10 governs. The identifier is replaced where it stands, and the note records that the replacement was made, not what it replaced. The same reasoning covers evidence: a report proving that a file holds an identifier describes the token, and does not reproduce it.

Required properties:

- `documentation.md` §10 states the rule:
  - a commit identifier already in a tracked file, the record included, is replaced where it stands by a name from §10's table;
  - a dated note beside it records that an identifier was replaced, without repeating it;
  - evidence of such a violation describes the token rather than quoting it.
- §6's withdrawal rule names this as its one exception.
- §9 records the amendment.
- In the same change, the rule is applied to every tracked file that carries a commit identifier.

##### D-13 §Implemented

2026-10-08 · Branch `host/initial-refactoring`.

`documentation.md` §6, §9 and §10. Applied to:

- the phase-3 entry of [`journal.md`](host/initial-refactoring/journal.md);
- the quotes in `reviewer-7` of [`implementation-reviewer.md`](reviews/host-initial-refactoring-r1/implementation-reviewer.md);
- the quotes in `integrity-5` of [`coherence-integrity.md`](reviews/host-initial-refactoring-r1/coherence-integrity.md).

Each place carries a dated note. A search of `.plan/`, `.agents/`, `README.md` and `CONTRIBUTING.md` for hexadecimal runs of seven characters or more finds no commit identifier. What remains is the signal mask in `I-2` and a qualified `agent:` token.

#### D-14 — A refused client reads the refusal whatever became of its request, and the bridge never waits on a refused peer

2026-10-09 · Accepted · Implemented 2026-10-09 · Addresses `F-25` · Alternatives in [`decisions.md`](host/initial-refactoring/decisions.md)

The bridge refuses a connection by writing an error response and closing the stream with whatever the peer sent left unread. That covers every refusal in the accept loop, which `D-15` lists, and a request turned away at shutdown. The response is always delivered. Its bytes are in the peer's receive queue before the close, and the reset that an unread request causes replaces only the end of stream that follows them. What a peer can lose is its chance to read them. A send that lands after the close fails with `EPIPE`, and `bridge.py` sent before it read, so it raised `Broken pipe` without looking at the refusal already waiting for it (`F-25`).

**Measured.** Round `host-initial-refactoring-r2` counted refusals lost by the real shim against sixteen held places: 1 of 118 on an idle machine and 22 of 39 under load. This ruling repeated that on the round's reviewed state and the repository's devcontainer, which has 12 CPUs and kernel 7.2.7. A scratch probe, not retained, ran 150 refusals in each of four configurations:

| Shim                                   | Idle       | Two busy loops per CPU |
| -------------------------------------- | ---------- | ---------------------- |
| Unchanged                              | 2 lost     | 62 lost                |
| Reads the response after a failed send | none lost  | none lost              |

**The client owes the delivery.** For the host to deliver it, the host would have to keep a refused stream open until the peer has sent. That means waiting on a peer that is outside the trust boundary (`CONTRIBUTING.md` §1.1):

- **On the accept thread**, one peer that never sends would stall every connection behind it.
- **On a thread or a timer per refusal**, the concurrency limit would stop bounding the bridge's work, because a refusal would cost what a worker costs.

The client already holds what it needs, because the response is in its own receive queue. HTTP gives a client the same duty, to watch for a response that arrives before it has finished sending.

Required properties:

- **The host's refusals are unchanged.** The host writes the error response and closes. It does not read the request and does not wait on the peer, in the accept loop and at shutdown alike. Each message is unchanged, and so are the protocol's bytes.
- **The shim reads the response whatever became of its send.** If sending the request fails, `bridge.py` still reads the response. A response that arrives is handled like any other, so a refusal reaches the user as its own message.
- **A failed send followed by no response is reported as the failed send**, as it is now. It is not reported as a malformed header.
- **The shim reads by the framing**: the header line, then `size` bytes. It never reads to end of stream, where a reset can stand in for the end of stream after a refusal.
- **Witnesses**, in `test_bridge.py`. Each drives `request_host` against a stand-in host on a real socket, and forces the order of events rather than racing it:
  - the stand-in writes an error response and closes before the shim sends, and the shim raises that response's message;
  - the stand-in closes without writing anything before the shim sends, and the shim reports the failed send.
- `test-architecture.md` §The layers and `README.md` §Tests and limitations say that the Python tests also cover the shim's reading of a response.

##### D-14 §Implemented

2026-10-09 · Branch `host/initial-refactoring`.

- **The shim.** `bridge.py`'s `request_host` keeps the `OSError` of a failed `sendall` and reads the header line all the same. If no header parses after a failed send, it raises the failed send. That covers an end of stream, a reset and a partial line. It still reads by the framing, `readline(4097)` then `read(size)`, so it never reads to end of stream.
- **The host** is unchanged.
- **Tests.** In `test_bridge.py`, `RequestHostTest` runs `request_host` against a stand-in host on a real socket. The stand-in accepts, writes its response or nothing, and closes, and only then does the shim's `connect` return, so the order is forced:
  - `test_should_report_a_refusal_that_arrived_before_the_request_was_sent` failed before the change with `BrokenPipeError`, and passes after it;
  - `test_should_report_the_failed_send_when_no_response_arrives` passes before and after, with `BrokenPipeError`.
- **Measured** with a scratch probe, not retained. The real shim ran 150 refusals against the release `serve` with sixteen places held, idle and under two busy loops per CPU. The changed shim lost none in either. The unchanged shim, in the same probe, lost none idle and 102 under load.

#### D-15 — `D-11`'s lifecycle, with `accept` retrying nothing and witnesses that hold whatever the runner ignores

2026-10-09 · Accepted · Implemented 2026-10-09 · Supersedes `D-11` · Addresses `F-6`, `F-10`, `F-11`, `F-21`, `F-26`, `F-27`, `F-28`, `F-29`, `F-30` · Alternatives in [`decisions.md`](host/initial-refactoring/decisions.md)

`D-11`'s lifecycle stands:

- the wake pipe;
- an accept loop with no timeout;
- scoped workers;
- refusals per connection;
- a cleanup order carried by types;
- inherited ignores kept;
- advertising stopped before the drain.

Round `host-initial-refactoring-r2` found three places where `D-11` asked for the wrong thing or did not ask for enough. This decision restates `D-11` with them corrected, so that the lifecycle contract can still be read from one entry.

**`accept` retries nothing (`F-29`).**

- **`D-11`'s retries rested on TCP's premise.** It retried `WouldBlock` because "the readiness was taken by a connection that vanished", and retried `ConnectionAborted` with it. On Linux `AF_UNIX`, a connection whose peer has gone stays queued, and `accept` returns it. The round saw this 600 of 600 times across three ways of leaving. No probe has produced `ECONNABORTED`, and the bridge has one acceptor. So once `poll` has reported the listener ready, `accept` returns a connection, or an error that concerns the process rather than one connection.
- **The `WouldBlock` arm really carried a signal.** A handled signal fails `poll` with `Interrupted`. The loop then fell through to an `accept` with nothing queued, got `EAGAIN`, and only the arm brought it back to `poll`. Without the arm, an idle bridge exits 1 on SIGTERM, 20 of 20 times in the round.
- **That dependency belongs at the `poll`.** An interrupted `poll` is retried as a `poll`, and the wake pipe is readable by then.
- **Probed in this ruling,** on a scratch copy of the round's reviewed state, with that change made and every `accept` retry removed:
  - 20 of 20 idle exits were clean, with status 0 and nothing on stderr, for each of SIGTERM, SIGINT and SIGHUP;
  - `serve` stayed up through 900 peers that connected and went away before they were accepted, 300 for each of the round's three ways of leaving, and then answered a request;
  - all 19 binary tests passed.

**Witnesses do not depend on the runner (`F-26`, `F-30`).**

- **The problem.** `serve` inherits the test runner's signal dispositions and keeps any that are ignored, as it should. A test that expects a signal to stop it therefore fails a correct bridge when the suite runs under `nohup`, or as a background job of a script.
- **What cannot fix it.** A POSIX shell cannot restore a disposition that was ignored when it started, and `unsafe` is confined to `sys` (`D-1`).
- **What does.** GNU coreutils' `env --default-signal` restores it, and coreutils is what `D-2` admits.
- **Probed.** Under `trap '' HUP INT`, all 8 server binary tests passed in a scratch copy that starts `serve` through `env --default-signal=TERM,INT,HUP`. In the same runner, the reviewed state's termination test failed after 10.02 s.
- **Coverage.** The inherited-ignore rule covers three signals, and only one of them was witnessed.

**The shutdown ordering is witnessed as well as built (`F-27`).** Two mutations that reverse `D-11`'s ordering passed every test. One drops the bridge after the join, and the other releases the lock before it. `F-11`, a Tier A defect, was exactly such an ordering, and a deterministic witness exists: a request whose tool is still running holds the drain open for as long as the test wants.

**Refusals (`F-25`).** The host's half of a refusal is unchanged: write the response, then close. The client's half is `D-14`.

Required properties:

- **No timed wake-up in the accept loop.** Shutdown begins at the loop's next wake after a handled signal.
- **Inherited ignored signals stay ignored.** The handled signals are SIGTERM, SIGINT and SIGHUP, each only if its disposition is not ignored when `serve` starts. A signal inherited as ignored stays ignored, and the bridge does not stop for it.
- **The handler only writes to the wake pipe.** It does async-signal-safe work and nothing else.
- **The bridge stops advertising before any worker is joined.** Once shutdown is requested, the watcher is stopped, then the socket path is removed, then the listener is closed. A connection that the kernel queued but `serve` never accepted is reset when the listener closes. A later connect fails at once.
- **A peer still sending its request is turned away.** If a connection's request line has not fully arrived when shutdown is requested, it gets an error response, `bridge is shutting down`, and is closed without waiting further. A connection whose request has arrived is answered, with its tools and its response write each within their own bounds (`D-4`, `D-6`).
- **`serve` returns only after every worker has finished.** No tool spawned for a connection outlives the process. The lock is released last, after every worker has finished. A successor started during the drain fails on the lock, exactly as it would against a running bridge.
- **A failure that concerns one connection never ends the server.** That covers:
  - reading its peer credentials;
  - making its stream non-blocking;
  - a refused UID;
  - the concurrency limit;
  - a failure to start its worker thread.

  Each gets an error response where one can still be written. The stream is then closed without reading the request and without waiting on the peer, and reading the response is the client's (`D-14`).
- **Which errors are retried:**
  - **`poll`.** `Interrupted` is retried, as a `poll`. In the accept loop, that retry is how a handled signal reaches the wake pipe, and the code says so where it retries.
  - **`accept`.** It is called only once `poll` has reported the listener ready, and none of its errors is retried. Each one ends `serve` with that error.
  - **Nothing else** in the signal or accept path answers `EINTR`, and the handlers are installed without `SA_RESTART`.
- **The cleanup order is a property of the types**, and it is stated where it is enforced.
- **The concurrency limit stays at 16**, and each existing refusal message is unchanged.
- **Witnesses, at the binary level:**
  - **Every `serve` a binary test starts begins with SIGTERM, SIGINT and SIGHUP at their default dispositions**, apart from a signal the test itself sets to ignored, so that no verdict depends on what the runner inherited. The layer uses GNU coreutils' `env --default-signal`, from 8.31 on, to do this, and `test-architecture.md` §The layers says that the layer needs it.
  - **An idle `serve` stops cleanly on each handled signal.** On each of SIGTERM, SIGINT and SIGHUP it exits 0, writes nothing to stderr, and leaves no socket. This is the witness for the interrupted-`poll` retry.
  - **Each inherited ignore is witnessed.** For each of SIGTERM, SIGINT and SIGHUP, a `serve` started with that signal ignored survives it and still answers a request. It then exits 0 on another handled signal.
  - **An idle peer is turned away.** With an idle connection open, after SIGTERM the socket path is gone, the idle peer receives `bridge is shutting down`, and `serve` exits 0 well inside one request phase.
  - **The ordering holds during a drain.** A request's tool is still running, and the test, not a fixed sleep, decides when it finishes. After SIGTERM, while `serve` is still running:
    - the socket path is gone, and a new connect fails at once;
    - a second `serve` on the same path fails on the lock.

    Once the tool finishes, `serve` exits 0, having reaped it. Each limb fails when its order is reversed.
  - **Peers that leave before they are accepted do not end `serve`.** After peers that connected and went away, a request is still served.
  - **The concurrency test that `F-14` §Ruling requires.** After the sixteen idle connections close, its exchange follows `D-14`'s client rule: it reads the response whether or not its send succeeded, and reads it by the framing. It therefore retries only on the limit refusal, and absorbs no I/O error.
  - **`should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal` still passes.**
- **The arms that `F-14` §Ruling waived stay waived.**
- **`README.md`** step 2 says, for each of the three signals, that one the bridge was started with ignored stays ignored. §Tests and limitations states what the witnesses above cover.

##### D-15 §Implemented

2026-10-09 · Branch `host/initial-refactoring`.

- **The accept loop.** `server::serve` retries an `Interrupted` `poll` with `continue`. The comment there says that this retry is how a handled signal reaches the wake pipe. `accept` is called once `poll` has returned with the wake pipe not ready, and any error it returns ends `serve` through `?`. The comment about a vanished connection is gone. The doc comment on `sys::wake_on_termination`, which says that `poll`'s callers retry `Interrupted`, is still true.
- **Unchanged from `D-11` §Implemented:** the dispositions read in `sys::wake_on_termination`, the handlers without `SA_RESTART`, `server::Bridge` dropping inside the scope, the lock dropped after it, and `Deadline::or_until_shutdown`.
- **The binary layer.** `Scratch::command` starts every binary through `env --default-signal`, with SIGTERM, SIGINT and SIGHUP restored apart from a signal the test ignores. That signal is passed to `env --ignore-signal`. `Scratch::serve_ignoring` takes only the signal.

Tests, all binary:

- `should_remove_the_socket_and_exit_0_on_a_termination_signal`, which now also asserts an empty stderr, is the idle-signal witness. In a scratch copy where an interrupted `poll` falls through to `accept`, it fails with exit status 1. So do the inherited-ignore, idle-peer and in-flight-reap tests.
- `should_keep_a_signal_ignored_when_serve_inherits_it_ignored` has one row per signal, each stopped by another handled signal. In a scratch copy that keeps an inherited ignore for SIGHUP only, the SIGTERM row fails, and so does the SIGINT row when run alone. The previous suite passes against that copy.
- `should_stop_advertising_and_keep_the_lock_while_draining_on_a_signal` is the ordering witness. Its stand-in tool runs until the test releases it. It fails under each of `F-27`'s mutations:
  - with the bridge dropped after the scope, the socket path outlives the signal;
  - with the lock released at the end of the loop, before the join, the successor `serve` starts. In this variant, no other test fails. A variant that releases the lock before the scope also fails `should_refuse_a_second_instance_while_the_first_holds_the_lock`.
- `should_keep_serving_after_peers_that_left_before_they_were_accepted` connects fifteen peers while `serve` is stopped by SIGSTOP. Five leave at once, five after part of a request, and five after a whole one. After SIGCONT, a request is served. There are fifteen so that the request never meets the limit of sixteen. A first version with thirty failed 9 of 40 parallel runs, because its request was refused and read to end of stream.
- `should_refuse_a_connection_over_the_limit_and_keep_serving` reads each response through `support::read_framed`, retries only the limit's refusal, and fails on any I/O error. It still fails under `F-12`'s `admit` mutation.
- `should_unadvertise_at_once_and_turn_away_an_idle_peer_on_a_signal` and `should_reap_an_in_flight_requests_tool_before_exiting_on_a_signal` pass unchanged.

The whole binary suite passes under `sh -c "trap '' HUP INT; exec cargo test …"`. The arms `F-14` §Ruling waived stay waived.

#### D-16 — The command line is read as OS strings by `lexopt`, and `D-12`'s contract stands

2026-10-09 · Accepted · Implemented 2026-10-09 · Addresses `F-35` · Alternatives in [`decisions.md`](packaging/decisions.md) · Plan in [`plan.md`](packaging/plan.md) §Stage 1

The owner asked for the hand-written parser in `cli::parse` to be replaced by `lexopt`, and for `D-12`'s contract to be kept. `lexopt` has no dependencies of its own. It reads arguments as `OsString`s, and it brings the getopt grammar: a value attached with `=`, `--` as the end of options, and clusters of short options. The hand parser reads `env::args()`, and that panics on an argument that is not UTF-8 (`F-35`): the binary exits 101 with a panic message, where `D-12` promises exit 1 and one line.

`D-12`'s rule that the binary "accepts `serve [--sync-text] [--allow-uid UID]…` and `sync-text`, and nothing else" is read here as a rule about invocations: which subcommands exist and which options `serve` takes. `--allow-uid=1001` is the conventional spelling of the published option, not another option, and refusing it would take code written against the library. It is accepted. `serve --` is the end of options with nothing after it, and is accepted for the same reason. `sync-text` takes no arguments at all, so `--` after it is an argument and is refused, as `D-12` says.

`lexopt`'s own error text is not shown to the user. `D-12` fixes three messages exactly, and the other refusals should read as the same program wrote them (`CONTRIBUTING.md` §12).

Required properties:

- **`D-12` holds unchanged.** The published invocations, the usage refusal with exit status 1, and the three messages `unknown argument: --bogus`, `missing UID` and `not a UID: <value>` are as `D-12` gives them. `D-12`'s two binary tests pass with their existing rows unmodified. They may gain rows.
- **No argument makes the binary panic, whatever its bytes.** Arguments are read as OS strings.
  - An argument that is not UTF-8 where a subcommand is expected prints the usage line and exits 1.
  - After `serve`, it exits 1 with one line naming the argument, decoded lossily.
  - An `--allow-uid` value that is not UTF-8 is reported as `not a UID: <value>`, decoded lossily.
- **The getopt spellings of the published options are accepted.** `--allow-uid=UID` is the same as `--allow-uid UID`. `--` ends `serve`'s options, and any argument after it is refused like any other operand. `serve --` alone starts the bridge.
- **`sync-text` with any argument, `--` included, prints the usage line and exits 1.**
- **Every other refusal after `serve` exits 1 with one line, in the binary's own wording, naming the argument at fault.** That covers a short option, a value attached to `--sync-text`, and an operand. The wording is the implementer's, in the form of `D-12`'s messages. No `lexopt` error text reaches stderr.
- **Witnesses:**
  - `D-12`'s two binary tables gain a row for each refusal above, the non-UTF-8 ones included. The non-UTF-8 rows fail before the change, with exit status 101.
  - A unit table in `cli`'s sibling test file (`D-2`) pins the accepted forms by the parsed command. It covers `--allow-uid=UID`, a repeated flag, options in either order, and `serve --`.
- **The dependency is accounted for.** `lexopt` is added with `cargo add` from `crates/host`. `CONTRIBUTING.md` §Language and platform names it beside `libc` and `serde_json`, with what it is for, and its change record carries the replaced sentence. `D-16` §Implemented records, before and after (`CONTRIBUTING.md` §15):
  - the release musl binary's size;
  - the clean release build time;
  - `cargo tree -e normal`.

##### D-16 §Implemented

2026-10-09 · Branch `host/initial-refactoring`.

- **The parser.** `main` passes `env::args_os().skip(1)` to `cli::parse`, which takes any iterator of OS strings. The subcommand is the first argument as given, compared as UTF-8, so `-- serve` prints the usage line and `sync-text --` is refused as `sync-text` with an argument. Only `serve`'s options go through `lexopt::Parser`.
- **Messages.** `D-12`'s three are unchanged. A short option, an unknown long option and an operand are each `unknown argument: <argument>`, decoded lossily. A value attached to `--sync-text` is `unknown argument: --sync-text=<value>`. A value attached to an unknown long option is not echoed: `serve --bogus=1` names `--bogus`, where the hand parser named `--bogus=1`. A short-option cluster is refused at its first letter. `parser.next()` fails only on an attached value left unread, and every arm reads it or refuses, so no `lexopt` error reaches stderr.
- **The binary tables** (`crates/host/tests/binary/cli.rs`). `Scratch::run` takes OS-string arguments. `D-12`'s rows are unchanged in content, written as `OsStr`s.
  - `should_print_usage_and_exit_1_for_an_unpublished_invocation` gains `\xff` and `sync-text --`. Before the change, the `\xff` row exited 101 with a panic; `sync-text --` already printed the usage line.
  - `should_exit_1_naming_the_argument_at_fault_for_malformed_serve_arguments` gains `serve \xff`, `serve --allow-uid \xff`, `serve -x`, `serve --sync-text=yes` and `serve -- extra`. Before the change, both non-UTF-8 rows exited 101 with a panic. `-x` and `--sync-text=yes` passed already. `serve -- extra` exited 1 naming `--`, not `extra`, so that row failed first rather than passing already as the plan expected.
- **The unit table** (`crates/host/src/cli/tests.rs`). `should_accept_the_getopt_spellings_of_the_published_invocations` pins the parsed command for `serve`, `serve --`, `--allow-uid=1001`, a repeated `--sync-text`, repeated `--allow-uid` in both spellings, both option orders, and `sync-text`. Before the change it failed at `serve --`, and the hand parser refused `--allow-uid=1001` with `unknown argument: --allow-uid=1001`.
- **Measured** (`CONTRIBUTING.md` §15), each from a clean export of the commit, `rustc 1.98.1`, 12 CPUs:

  | | Before, the change that mints `D-16`–`D-19` | After |
  | --- | --- | --- |
  | Release musl binary | 611,072 bytes | 623,360 bytes (+12,288, +2.0%) |
  | `.text` | 459,406 bytes | 467,598 bytes (+8,192) |
  | Clean release build | 4.32–4.41 s, three runs | 4.39–4.42 s, three warm runs; a first run after `cargo fetch` took 6.48 s |
  | `cargo tree -e normal` | `libc`, `serde_json` (with `itoa`, `memchr`, `serde_core`, `zmij`) | the same, and `lexopt` 0.3.2 with no dependencies |

  `cargo bloat` attributes the `.text` delta of about 7.8 KiB by function. `lexopt`'s own code is 3.1 KiB, chiefly `Parser::next` at 1.9 KiB and `Error`'s `Display` at 0.6 KiB, the latter linked through `?` and never reached. The parser inlined into `main` adds 1.3 KiB. About 2.4 KiB of `Debug` formatting comes in with `lexopt`, for the state named in its internal `panic!`. `env::Args` and its drop glue go, about 0.5 KiB. The file grows by three pages. `cargo tree --duplicates` prints nothing.

#### D-17 — `serve`'s default socket is in the user's data directory, wherever the binary is

2026-10-09 · Accepted · Alternatives in [`decisions.md`](packaging/decisions.md) · Plan in [`plan.md`](packaging/plan.md) §Stage 2

Without `CLAUDE_CLIPBOARD_SOCKET`, `server::serve` puts the socket and its lock beside the executable. That works only because `setup-host.sh` installs the binary into `$HOME/.local/share/claude-clipboard`, which the user can write to. A packaged binary lives in `/usr/bin` (`D-18`), and there opening the lock fails with a permission error before anything else happens.

The directory `setup-host.sh` already uses becomes the default for every install. For an install made by `setup-host.sh`, nothing changes. A binary run from `dist/` or `target/` now puts its socket in the home directory instead of beside itself. Nothing is released, so no compatibility is owed for that (`CONTRIBUTING.md` §8). On a fresh package install the directory does not exist yet, and `serve` is the only thing the user runs, so `serve` creates it.

Required properties:

- **Without `CLAUDE_CLIPBOARD_SOCKET`, the socket is `$HOME/.local/share/claude-clipboard/clipboard.sock`,** with the lock beside it as today, whatever the executable's location.
- **If that directory does not exist, `serve` creates it, and its parents with it.** The directory `serve` creates is mode 0755 whatever the umask, because a container UID allowed by `--allow-uid` has to be able to reach the socket. An existing directory's mode is left alone.
- **With `CLAUDE_CLIPBOARD_SOCKET` set, behaviour is unchanged,** and no directory is created.
- **With `HOME` unset or empty and no override, `serve` exits 1** with one line naming both variables.
- **The shim's resolution of the socket is unchanged.**
- **Witness:** a binary test runs `serve` with `HOME` in its scratch directory, no `CLAUDE_CLIPBOARD_SOCKET`, and umask 077. It finds the socket at the default path, and the created directory at mode 0755. Before the change it fails, because the socket is beside the test binary.
- **`README.md`** says where the host's socket is, and the autostart line works for both installs.

#### D-18 — The `claude-clipboard` packages carry the host binary and the container shim, and require only `wl-clipboard`

2026-10-09 · Accepted · Alternatives in [`decisions.md`](packaging/decisions.md) · Plan in [`plan.md`](packaging/plan.md) §Stage 2

The owner asked for installable x86_64 `.deb` and `.rpm` packages that carry the container shim, declare the runtime dependencies, and need no development tools and no Python on the host. The host binary is static musl, so it needs no C library. It runs `wl-paste` for every request and `xsel` only for `--sync-text`. The shim runs only in the container, so Python is a property of the container, never of the host.

A package cannot write into a home directory, so the files it installs and the socket now live apart: the shim under `/usr/share`, root-owned, and the socket in the user's directory (`D-17`). A container therefore mounts two directories where a `setup-host.sh` install mounts one. The package's directory is mounted without relabelling, because `z` would relabel files the package owns, and the package database and `restorecon` would then disagree with the mount. Whether a container under enforcing SELinux may execute the shim from a `usr_t` path has not been observed. A 2016 note by the policy's author says the container domain may read and execute most of `/usr`. A Fedora report shows `entrypoint` refused for a `usr_t` script, but in that report the script was the container's entrypoint, which the shim never is. This is settled by observation on Fedora before the README claims it.

`setup-host.sh` stays as the install from source, and its one-mount layout is unchanged.

Required properties:

- **Two packages per build, from one manifest:** a `.deb` for `amd64` and an `.rpm` for `x86_64`. Both are named `claude-clipboard`, and both carry the workspace version. The rpm release is `1`. Both formats come from the one manifest, so they carry the same files.
- **Contents, exactly:**
  - `/usr/bin/claude-clipboard-host`, mode 0755: the release musl binary of the same build whose tests passed (`D-19`);
  - `/usr/share/claude-clipboard/bridge.py`, mode 0755, byte-identical to the tracked `bridge.py`, shebang included;
  - `/usr/share/claude-clipboard/bin/wl-paste` and `/usr/share/claude-clipboard/bin/xclip`, each a relative symlink to `../bridge.py`;
  - `README.md` as the package's documentation, at the format's documentation path for the package;
  - the package owns `/usr/share/claude-clipboard` and its `bin`, so that removing the package removes them.
- **Dependencies, exactly:** it requires `wl-clipboard` and recommends `xsel`. Nothing else is declared or generated:
  - no Python;
  - no C library;
  - no interpreter dependency derived from the shim's shebang;
  - no build tool.
  
  The format's own `rpmlib(…)` capabilities are not dependencies in this sense.
- **No maintainer scripts.** Installing starts, enables and configures nothing.
- **The container side of a package install,** published under `CONTRIBUTING.md` §4:
  - `/usr/share/claude-clipboard` is mounted at `/opt/host-clipboard`, read-only, and never with `z` or `Z`;
  - `$HOME/.local/share/claude-clipboard` is mounted at `/run/host-clipboard`, read-only, with `z`;
  - the container sets `CLAUDE_CLIPBOARD_SOCKET=/run/host-clipboard/clipboard.sock`;
  - `PATH` gains `/opt/host-clipboard/bin`, as today.
- **`README.md`** gives the package install and its layout, and keeps `setup-host.sh` with its one-mount layout. It does not say that the package layout works under enforcing SELinux until the owner's Fedora check has shown it. If the check shows a denial, it goes to the architect with its AVC record. Neither `/usr/share/claude-clipboard` nor the host's policy is relabelled to work around it.
- **No license value until `Q-2` is answered.** If a format refuses to build without one, the work stops, and `Q-2` goes to the owner.

#### D-19 — One Docker build runs every gate and makes every product, and the workflow only invokes it

2026-10-09 · Accepted · Alternatives in [`decisions.md`](packaging/decisions.md) · Plan in [`plan.md`](packaging/plan.md) §Stage 2

`crates/host/Dockerfile` tests and builds the binary, and it has never been run: the devcontainer has no Docker CLI, and the journal has said so since phase 1. [`test-architecture.md`](../.agents/docs/test-architecture.md) §CI policy already names the gate that CI must run once it exists. The owner asked for GitHub Actions that produce the packages, check that they install and remove, and validate the Docker builder. Building no host tooling means that the packages, and every check on them, come from Docker alone.

All of it is therefore one Docker build. The workflow is a thin caller, so that anyone with Docker reruns exactly what CI ran, and the Docker builder is validated because it is the only builder. Each image is pinned to an exact version, because a gate that fails on an upstream release teaches people to re-run it. The workflow uses no third-party action. GitHub's own actions are pinned to exact release tags rather than commit identifiers, which [`documentation.md`](../.agents/docs/documentation.md) §10 keeps out of tracked files.

Required properties:

- **One Dockerfile at the repository root replaces `crates/host/Dockerfile`.** `.dockerignore` admits exactly what its stages read.
  - The README's build command still runs the musl tests and then exports `dist/claude-clipboard-host`.
  - A documented target exports the `.deb` and the `.rpm` into `dist/`.
- **Every image is pinned to an exact version:** the Rust toolchain, Python, Node, the packaging tool, Fedora and Debian. None is `latest`, a major version alone, or a release codename alone. The packaging tool is nfpm, from its official image.
- **Every gate in `test-architecture.md` §CI policy is a target of the build:**
  - `cargo fmt --check`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo test --workspace`;
  - the musl-target test;
  - the Python suite;
  - the guard's `node --test` suite.
  
  Each runs alone with `docker build --target …`, and with nothing on the machine but Docker and the checkout.
- **The packages are made from the binary of the build that ran the musl tests.** It is not compiled a second time.
- **An install-and-remove check for each format is a target, in a pinned image of a current release:** Fedora for the `.rpm`, and Debian stable for the `.deb`. Each check:
  - installs the package from the local file with the distribution's resolver, which pulls `wl-clipboard`, and `xsel` by default;
  - asserts that the declared dependencies are exactly `D-18`'s, and that the install pulled no Python;
  - asserts every path, mode and link target in `D-18`, and that `bridge.py` is byte-identical to the tracked file;
  - verifies the installed files against the package database;
  - runs `claude-clipboard-host` with no arguments, and gets the usage line and exit status 1 (`D-12`);
  - removes the package, and asserts that nothing `D-18` installed remains, its directories included.
- **The Fedora check also runs the packaged layout end to end,** as an unprivileged user whose `$HOME/.local/share/claude-clipboard` does not exist:
  - `serve` starts, and creates the directory and the socket (`D-17`);
  - the packaged `wl-paste`, run with a Python installed after the dependency assertions, lists types over that socket and prints the host's own error for the missing Wayland session, not a connection error;
  - SIGTERM ends `serve` with exit status 0 and the socket gone.
- **The workflow, under `.github/workflows/`:**
  - runs on a push to any branch, on a pull request and on manual dispatch;
  - has `contents: read` permission and uses no secret;
  - runs on a runner label with a version, not `-latest`;
  - uses only GitHub's own actions, each pinned to an exact release tag;
  - contains no build, test or check logic of its own, so that every job is a `docker build` of a target a reader can run locally;
  - uploads the `.deb` and the `.rpm` as artifacts only from a run in which every gate and both install checks passed.
- **Documentation:**
  - `test-architecture.md` §CI policy states the gate that is installed, and its change record carries the replaced text;
  - [`documentation.md`](../.agents/docs/documentation.md) §8 and `AGENTS.md` §Where things are name the Dockerfile, the packaging directory and the workflow;
  - `README.md` gives the build commands and the package install.
- **Evidence:** `D-19` §Implemented names the workflow run on the pushed head of the branch, by its run number, in which every job passed. The Docker CLI is absent from the devcontainer, so that run is the validation of the builder.

---

## Findings

#### F-1 — Tool capture is capped near 14 MiB/s, so images close to the limit time out

2026-10-06 · Tier A · Settled 2026-10-06 by `D-4` · Evidence `I-1`

`capture` in `crates/host/src/main.rs` reads into a 16 KiB chunk and then sleeps 1 ms on every loop iteration, data or not. Measured: 1 MiB in 71 ms, 32 MiB in 2.2 s, 63 MiB in 4.4 s. Under the 4 s bound that `wl-paste` reads get, any image above roughly 56 MiB fails with `wl-paste timed out`, although the documented limit is 64 MiB, and a 30 MiB screenshot costs about 2 s of paste latency for nothing.

Required property: capture time is bounded by the producer, not by the host's wait loop, and an image at the 64 MiB limit is deliverable within the existing bound.

#### F-2 — A slow peer holds a worker slot without bound

2026-10-06 · Tier A · Settled 2026-10-06 by `D-6` · Evidence `I-2`

`serve` sets 12 s read and write timeouts on each accepted stream, and `std` applies them per syscall. `process_request` reads the request line with `read_until`, which issues one read for each byte as it arrives. Measured: a peer sending one byte every 10 s still held its connection after 30 s, with no response sent. Allowing for the 4096-byte request limit, one such peer holds a slot for up to about 13 h. Sixteen of them make the bridge answer everyone with `too many concurrent clipboard requests`. The response write has the same shape against a peer that reads a 64 MiB payload slowly. The peer is outside the trust boundary, so any bytes at any timing are in the contract.

Required property: every phase of a connection completes or fails within a total bound independent of the peer's pacing.

#### F-3 — The text-sync watcher outlives a bridge that is killed

2026-10-06 · Tier A · Settled 2026-10-06 by `D-8` · Evidence `I-2`

With `--sync-text`, `serve` starts `wl-paste --watch claude-clipboard-host sync-text` in its own process group, and only the guard's `Drop` kills it. Measured: after SIGKILL to the bridge, the watcher was still running with parent PID 1. It keeps writing every text copy into X11, and a restarted bridge starts a second watcher beside it. SIGHUP, which a closing terminal sends, has the same default action of terminating without running destructors; that case follows from the same mechanism and was not run separately.

Required property: the watcher never outlives the serving process, however that process ends.

#### F-4 — A non-UTF-8 entry in the clipboard's type listing fails every image read

2026-10-06 · Tier A · Settled 2026-10-06 by `D-5` · Evidence `I-2`

`respond_with` and `sync_text` validate the entire `wl-paste --list-types` output as UTF-8 before looking at any line. Measured: a listing of `image/png` plus a line containing byte `0xff` makes a read of `image/png` fail with a `Utf8Error`. The listing comes from whichever application owns the clipboard, so this is the environment's input.

Required property: a listing line that is not a supported type is ignored, whatever its bytes.

#### F-5 — The response header is written as one syscall per JSON token

2026-10-06 · Tier C · Settled 2026-10-06 by `D-6` · Evidence `I-2`

`write_response` hands the unbuffered `UnixStream` to `serde_json::to_writer`. Traced: a successful response's header costs 14 `sendto` calls — `{`, `"`, `ok`, `"`, `:`, … — before the single call that sends the payload.

Required property: a response leaves in a number of writes independent of the header's content, without copying the payload.

#### F-6 — Shutdown waits on a timer, abandons in-flight workers, and a per-connection failure can stop the server

2026-10-06 · Tier A · Settled 2026-10-06 by `D-7` · Evidence: code reading, [`analysis.md`](host/initial-refactoring/analysis.md) §Server lifecycle

Three defects in `serve`, `crates/host/src/main.rs`:

- **Shutdown waits on a timer.** The accept loop polls with a 1000 ms timeout and checks a flag. A signal therefore takes up to 1 s to act, and an idle bridge wakes once a second.
- **In-flight workers are abandoned.** `serve` returns without waiting for its worker threads. Process exit ends them without running their `ChildGuard`s, so a tool spawned for a connection is left to init.
- **One connection can stop the server.** `?` on `set_read_timeout` and `set_write_timeout`, and on any `accept` error other than `WouldBlock`, ends `serve`. That includes the `EINTR` a signal causes when it lands between `poll` and `accept`, because the handlers are installed without `SA_RESTART`. `thread::spawn` panics instead of returning an error.

The second and third were not reproduced, because their windows are narrow. The mechanisms are in the source.

Required property: shutdown is prompt and event-driven, `serve` returns only after every worker has finished, and a failure that concerns one connection never ends the server.

> 2026-10-08 — The third bullet's `EINTR` clause is withdrawn (`F-21`). The listener was non-blocking in the initial implementation, and a non-blocking `accept` cannot fail with `EINTR`, so that path never existed. The withdrawn text read: _"That includes the `EINTR` a signal causes when it lands between `poll` and `accept`, because the handlers are installed without `SA_RESTART`."_ It is left standing above only because this entry is append-only. The rest of the bullet stands: `?` on the socket timeouts, a fatal non-`WouldBlock` `accept` error such as `ECONNABORTED`, and the `thread::spawn` panic. `D-11` supersedes `D-7`'s clause that rested on it.

> 2026-10-09 — The example in the previous note is withdrawn (`F-29`). It read: _"a fatal non-`WouldBlock` `accept` error such as `ECONNABORTED`"_. On Linux `AF_UNIX`, a peer that has gone stays queued and is accepted. No probe has produced `ECONNABORTED` there, so an `accept` error was never the per-connection failure that bullet counted. The limbs about the socket timeouts and `thread::spawn` stand. `D-15` retries no `accept` error.

#### F-7 — The host is one file with stringly-typed values, scattered `unsafe` and inline tests

2026-10-06 · Tier C · Settled 2026-10-06 by `D-1`, `D-2`, `D-3`

The whole host is `crates/host/src/main.rs`:

- Protocol policy, child management, server lifecycle, text sync and 13 tests share one file.
- MIME types, operations and `wl-paste` arguments are `&str`, so a mistyped MIME string or flag compiles.
- Nine `unsafe` blocks in production code carry no `SAFETY` comment.
- `ChildGuard(Child, bool)` encodes "reaped" as a `bool`.
- The test seam `respond_with` takes a closure over `wl-paste` argument arrays, so tests spell wl-paste flags.
- `flock` goes through `libc`, although `std` provides it.

None of this is visible at runtime. It is what makes `F-1` to `F-6` hard to see and expensive to fix.

Required property: see the required properties of `D-1`, `D-2` and `D-3`.

#### F-8 — `handle-stdio` is an unpublished subcommand with no caller

2026-10-06 · Tier C · Settled 2026-10-06 by `D-9`

`main` accepts `handle-stdio`, which serves one request from stdin to stdout. It is not in the usage line or the README, and nothing in the repository invokes it.

Required property: the binary's subcommands are exactly the published ones.

#### F-9 — The server lifecycle, text-sync sequence and large payloads have no test

2026-10-06 · Tier C · Settled 2026-10-06 by `D-2`, `D-4`, `D-10`

No test exercises:

- binding, the lock that excludes a second instance, replacement of a stale socket, refusal of a socket path the user does not own, or cleanup on a signal;
- the text-sync sequence beyond its type filter;
- a payload larger than 22 bytes.

The last gap is why `F-1` went unseen. README §Tests and limitations says the Rust tests cover "the Unix socket"; they cover a socket pair, not the server.

Required property: each of the above has a test in the layer that can reach it.

#### F-10 — A bridge started with SIGHUP ignored now exits when its terminal closes

2026-10-07 · Tier A · Routed to the architect (`D-7`) · Ruled 2026-10-08 by `D-11` · Settled 2026-10-09 by `D-11` §Implemented · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-1`)

`sys::wake_on_termination` installs a SIGHUP handler unconditionally and never reads the old action. A disposition inherited as ignored, as under `nohup`, was kept by the initial implementation and is now replaced, so a bridge that used to outlive its terminal shuts down with it.

Required property: a signal the bridge inherited as ignored stays ignored, or the record states that ignoring SIGHUP no longer detaches the bridge.

#### F-11 — After a termination signal the socket stays bound and new connections queue unserved

2026-10-07 · Tier A · Routed to the architect (`D-7`) · Ruled 2026-10-08 by `D-11` · Settled 2026-10-09 by `D-11` §Implemented · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-2`)

On the wake byte, `serve` joins every worker inside `thread::scope`, and only after that drops `Bridge`, which removes the path and closes the listener. While the workers are joined, the kernel queues new connections that nothing will accept. With one idle connection open, the process exited 11.9 s after SIGTERM, and a client that connected meanwhile got no reply and then a reset.

Required property: once shutdown is requested, a new connection is refused promptly or served, and `D-7` states which ordering "as soon as the signal is delivered" means.

#### F-12 — Running-server coverage is claimed for wiring that no test pins

2026-10-07 · Tier B · Routed to the implementer · Settled 2026-10-09 in [`plan.md`](host/initial-refactoring/plan.md) §Phase 5, step 5.5: the concurrency limit is pinned through the running server, and `README.md` and `test-architecture.md` state the peer UID check and the connection deadlines as proved at the function level · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-3`)

`README.md` §Tests and limitations and `test-architecture.md` §The layers claim that tests cover the running server's peer-UID check, its concurrency limit and its per-connection deadline. Two mutations survived the full suite: replacing the `admit` call in `serve` with an unconditional slot, and raising `PHASE` to 3600 s.

Required property: every coverage claim can be falsified by mutating the site it claims, or the text says the property is proved only at the function level.

#### F-13 — A failed capture with input can outlive its deadline

2026-10-07 · Tier C · Routed to the architect (`D-4`) · Ruled 2026-10-08 in `D-4` §Adjudicated, a defect against `D-4` · Settled 2026-10-09 by `D-4` §Remediated · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-1`)

When a tool exits non-zero while another member of its group holds stdin open, `capture` joins the stdin writer after dropping a reaped `Tool`. That drop signals nothing, so the join waits for the group member to exit. A probe took 3.01 s against a 200 ms deadline; the initial implementation returned in 2.45 ms. `D-4`'s "one deadline bounds the whole capture" and "a reaped child is never signalled" conflict on this path. No production trigger is known, because `xsel -i` reads all of its input before it forks.

Required property: a capture's total duration, the stdin join included, is bounded on its failure paths.

#### F-14 — `F-6`'s per-connection limb has no test that drives the accept loop

2026-10-07 · Tier C · Routed to the owner or the architect · Ruled 2026-10-08 (§Ruling below) · Settled 2026-10-09 by `should_refuse_a_connection_over_the_limit_and_keep_serving`, with the waived arms named in `D-11` §Implemented · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-2`)

Plan step 3.5 requires a test showing that a per-connection failure leaves the loop running, and none exists. `admit`'s tests pin refusal messages and slot counting, which were not the defect. The journal acknowledges the gap.

Required property: either a test shows that a per-connection failure leaves `serve` accepting, or the waiver of that exit criterion is recorded with who granted it.

##### F-14 §Ruling

2026-10-08 · Architect · A test is owed. The limb is not waived as a whole.

**The test.** The concurrency refusal can be reached through the running binary with nothing provisioned:

1. Sixteen connections that send nothing hold every place.
2. A seventeenth gets `too many concurrent clipboard requests`.
3. Once the sixteen close, a request is served.

This test also pins the refusal arm of the accept loop in `server::serve`. A refused UID and a failure to make the stream non-blocking go through that same arm, so a regression that turns the arm into a `return`, or puts a `?` in front of `admit`, fails it. The same test is `F-12`'s witness for the wiring of the concurrency limit.

**Waived by the architect, on this date,** from plan step 3.5's criterion: the arms that no test can provoke without privilege or fault injection.

- **A failure to read the peer's credentials.**
- **A refused UID**, which needs a second account.
- **A failure to make the stream non-blocking.**
- **A failure to start a worker thread.**

Their handling is structural, and every arm but the last shares the arm the test above pins. `EINTR` on `accept` drops out of the limb altogether (`D-11`, `F-21`).

#### F-15 — Three binary tests start `serve` with no path that ends it

2026-10-07 · Tier C · Routed to the implementer · Settled 2026-10-09 in [`plan.md`](host/initial-refactoring/plan.md) §Phase 5, step 5.1: `Scratch::run` and `Scratch::serve` in `crates/host/tests/binary/support.rs` kill the binary and fail the test on every outcome · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-3`)

`Scratch::run` waits for the binary to exit, and three tests use it to start `serve`, expecting it to refuse. A regression that lets `serve` start makes `cargo test` and the image build hang rather than fail, which breaks `D-2`'s termination property.

Required property: every binary test that starts `serve` ends it and reports a failure on every outcome.

#### F-16 — `D-6` gives the error header's key order differently from the bytes the host sends

2026-10-07 · Tier C · Routed to the architect (`D-6`) · Settled 2026-10-08 by a non-substantive correction of `D-6` · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-4`, `integrity-4`)

`D-6` gives `{"ok":false,"size":0,"error":…}`. The host emits sorted keys, `{"error":…,"ok":false,"size":0}`, and always has.

Required property: the record describes the header's wire form as the host emits it, or says that field order is not part of the contract.

#### F-17 — A graceful shutdown kills an in-flight text sync

2026-10-07 · Tier C · Routed to the architect (`D-8`) · Ruled 2026-10-08 in `D-8` §Adjudicated · Settled 2026-10-09 by `D-8` §Remediated · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-5`)

On SIGTERM, SIGINT or SIGHUP, the `Bridge` drop kills the watcher's process group, and every running `sync-text` is in that group. A probe confirmed this. `D-8` and the `clipboard::watch` doc comment say a running sync runs to completion. That holds only on parent death. The behaviour is unchanged from the initial implementation.

Required property: the record and the comment state truthfully what happens to an in-flight sync on each way the bridge ends.

#### F-18 — Malformed `serve` options do not print `D-9`'s usage line

2026-10-07 · Tier C · Routed to the architect (`D-9`, `D-3`) · Ruled 2026-10-08 by `D-12` · Settled 2026-10-09 by `D-12` §Implemented · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-6`)

`serve --bogus`, `serve --allow-uid alice` and `serve --allow-uid` exit 1 with a specific message, unchanged from the initial implementation as `D-3` requires. `D-9` says "any other invocation prints the usage line".

Required property: the record states one rule for what a malformed `serve` invocation prints, and the binary follows it.

#### F-19 — The journal cites a commit range as provenance

2026-10-07 · Tier C · Routed to the record owner · Settled 2026-10-08 by `D-13` · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-7`, `integrity-5`)

The phase-3 entry of [`journal.md`](host/initial-refactoring/journal.md) gives the span in which a defect existed as a range of commit identifiers. `documentation.md` §10 forbids that in a tracked file.

Required property: the record names the affected states in §10's vocabulary, not by commit identifiers.

#### F-20 — `serve` can pass `Deadline` a blocking stream

2026-10-07 · Tier C · Routed to the implementer · Settled 2026-10-09 in [`plan.md`](host/initial-refactoring/plan.md) §Phase 5, step 5.9: the refusal site in `server::serve` states the assumption it relies on · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-6`)

If `set_nonblocking` fails on an accepted stream, `serve` still writes the refusal through `Deadline::new`, whose documented precondition is a non-blocking stream.

Required property: any path that hands `Deadline` a stream meets its precondition, or the call site states the assumption it relies on.

#### F-21 — `SA_RESTART` and the `accept` loop's `Interrupted` retry guard against a failure that cannot occur

2026-10-07 · Tier C · Routed to the architect (`D-7`, `F-6`) · Ruled 2026-10-08 by `D-11` · Settled 2026-10-09 by `D-11` §Implemented · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`der-1`)

The listener has always been non-blocking, and a non-blocking `accept` never sleeps, so a signal cannot interrupt it with `EINTR`. A probe saw 0 `EINTR` in 200,000 non-blocking `accept` calls across 7.4 million handler runs; the blocking control returned `EINTR` on all 2,000 calls. `F-6`'s third bullet, `D-7`'s `EINTR` clause, `SA_RESTART` and its comment all rest on the opposite premise.

Required property: every mechanism in the signal and accept path, and the comment that justifies it, describes a failure the serving process can actually meet.

#### F-22 — `Deadline`'s doc comment restates a withdrawn premise

2026-10-07 · Tier C · Routed to the implementer · Settled 2026-10-09 in [`plan.md`](host/initial-refactoring/plan.md) §Phase 5, step 5.4: the doc comment on `server::Deadline` gives the adjudicated premise · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`der-2`)

The comment begins "A socket timeout bounds a single call", the premise `D-6` §Adjudicated withdrew.

Required property: the stated reason for `Deadline` matches the adjudicated premise.

#### F-23 — The error alias carries `Send + Sync` that no caller needs

2026-10-07 · Tier C · Routed to the implementer · Settled 2026-10-09 in [`plan.md`](host/initial-refactoring/plan.md) §Phase 5, step 5.9: the alias in `main.rs` has no `Send + Sync` · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`cleanup-1`)

No error typed with `crate::Result` crosses a thread, and the crate builds with the bounds removed.

Required property: the error type's bounds are no wider than some caller needs.

#### F-24 — The `handle-stdio` refusal test repeats the usage-refusal test

2026-10-07 · Tier C · Routed to the architect (`D-9`) · Ruled 2026-10-08 by `D-12` · Settled 2026-10-09 by `D-12` §Implemented · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`cleanup-2`)

`should_refuse_handle_stdio_as_an_unpublished_invocation` asserts the same behaviour as `should_print_usage_and_exit_1_for_an_unpublished_invocation`, and its clipboard stand-in plays no part in its assertion. `D-9` §Implemented and plan step 7 name it as `D-9`'s witness, so whether it should stay is a question about `D-9`.

Required property: the refusal of unknown invocations is pinned where the record requires it, and no test carries setup its assertion does not depend on.

#### F-25 — A refusal is lost when the peer's send lands after the bridge closes, and the shim sends first

2026-10-09 · Tier A · Routed to the architect (`D-11`; host or shim) · Ruled 2026-10-09 by `D-14`: the shim owes it, and the host is unchanged · Settled 2026-10-09 by `D-14` §Implemented · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`reviewer-1`, `integrity-1`, `cleanup-2`)

`serve` writes a refusal and closes the stream with the request unread. This happens for a refused UID, for the concurrency limit, for the other refusals in the accept loop, and for a request turned away at shutdown with `bridge is shutting down`. `bridge.py`'s `request_host` sends with `sendall` before it reads anything. If the close comes first, the send fails with `EPIPE`, and the shim prints `wl-paste: [Errno 32] Broken pipe` without reading the refusal already waiting in its receive queue.

With the real shim, 1 of 118 refusals was lost on an idle machine and 22 of 39 under CPU load. The refusal's bytes always arrive before any reset. The journal's account, that the reset comes on the read and only under parallel load, is wrong about the mechanism. The behaviour predates the branch.

Required property: a peer that is refused, or turned away at shutdown, learns why, whether its request reaches the host before or after the host closes — or the record states that a refusal may surface as a broken connection.

#### F-26 — The termination-signal binary test fails a correct bridge when the runner inherited an ignored signal

2026-10-09 · Tier B · Routed to the architect (`D-2`, `D-11`) · Ruled 2026-10-09 by `D-15`: every `serve` a binary test starts gets default dispositions through `env --default-signal` · Settled 2026-10-09 by `D-15` §Implemented · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`der-1`, `reviewer-2`, `integrity-3`)

`should_remove_the_socket_and_exit_0_on_a_termination_signal` expects SIGTERM, SIGINT and SIGHUP each to end `serve`. That is `D-7`'s unconditional property. `serve` inherits the runner's signal dispositions and, as `D-11` requires, keeps any that were ignored. Under `trap '' HUP`, under `trap '' INT`, or as a background job of a non-interactive shell, the test fails after 10 s with `serve did not exit`, and the message names no cause.

Required property: the binary layer's verdict on signal handling does not depend on the dispositions its runner inherited, or the precondition is stated and its failure names it.

#### F-27 — Two of `D-11`'s shutdown-ordering properties have no witness

2026-10-09 · Tier B · Routed to the implementer · 2026-10-09: tests are owed, not a statement, as `D-15`'s witnesses · Settled 2026-10-09 by `should_stop_advertising_and_keep_the_lock_while_draining_on_a_signal`, in `D-15` §Implemented · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`integrity-2`)

Each of two mutations passes all 19 binary tests: dropping `bridge` after the scope instead of inside it, and releasing the lock before the join. `D-11` requires both that "the bridge stops advertising before any worker is joined" and that "the lock is released last". `README.md` lists the first among what the binary tests cover.

Required property: each shutdown-ordering property the register states can be falsified by a test that fails when the order is reversed, or the record says it is held by construction only.

#### F-28 — The concurrency test's retry gives the wrong mechanism and tolerates any I/O error

2026-10-09 · Tier C · Routed to the implementer, after `F-25` is ruled · `F-25` ruled 2026-10-09: the exchange follows `D-14`'s client rule and absorbs no I/O error, as `D-15` requires · Settled 2026-10-09 in `D-15` §Implemented · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`reviewer-3`, `der-3`, `cleanup-2`)

The retry's comment in `should_refuse_a_connection_over_the_limit_and_keep_serving` says a refused connection "can reset it before the refusal is read". In fact an exchange fails in one of two ways. The send fails with `EPIPE`, or `read_to_end` hits `ECONNRESET` after it has already received the refusal. The loop retries every I/O error.

Required property: the retry tolerates the failures that actually occur, and no others, and its comment states them as they occur.

#### F-29 — The accept loop's retries rest on a TCP premise, and the `WouldBlock` arm's real job is unstated

2026-10-09 · Tier C · Routed to the architect (`D-11`) · Ruled 2026-10-09 by `D-15`: an interrupted `poll` is retried as a `poll`, and no `accept` error is retried · Settled 2026-10-09 by `D-15` §Implemented · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`der-2`)

`D-11` and the code say `WouldBlock` is retried because "a connection that vanished" took the readiness. On Linux `AF_UNIX`, a vanished peer is still accepted: 600 of 600 times across three ways of leaving. No evidence was found that `ConnectionAborted` can occur. The `WouldBlock` arm does carry a different load, which nothing states. A signal interrupts `poll`, the loop falls through to an `accept` that returns `EAGAIN`, and that arm sends it back to `poll`. Without the arm, an idle bridge exits 1 on SIGTERM, 20 of 20 times.

Required property: each retry arm, and `D-11`'s statement of it, names a failure that a non-blocking `AF_UNIX` listener in this process can actually meet, and the dependency of a clean exit on a signal is stated where it is enforced.

#### F-30 — Only SIGHUP's inherited ignore is witnessed, and `README.md` names it as the only exception

2026-10-09 · Tier C · Routed to the implementer · 2026-10-09: the witness for each signal and the README's wording are owed by `D-15` · Settled 2026-10-09 in `D-15` §Implemented, and `README.md` step 2 names all three signals · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`integrity-4`)

`D-11` keeps an inherited ignore for all three signals. The only test ignores SIGHUP, and a mutation that honours the rule for SIGHUP alone passes. `README.md` step 2 says Ctrl+C or SIGTERM stops the bridge, and gives SIGHUP as the only inherited exception.

Required property: the README's account of when the bridge keeps running agrees with `D-11` for each signal, and a coverage claim is backed for each signal it covers.

#### F-31 — `serve_ignoring` takes parameters its only caller fixes

2026-10-09 · Tier C · Routed to the implementer, with `F-30` · Settled 2026-10-09: `Scratch::serve_ignoring` takes only the signal, which the suite varies · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`cleanup-3`)

`Scratch::serve_ignoring(signal, args)` has one caller, which passes `"HUP"` and `&[]`.

Required property: the helper's parameters match what the suite varies, or the suite varies them.

#### F-32 — `documentation.md` §5.1's model comment states a constraint `D-4` removed

2026-10-09 · Tier C · Routed to the owner of `documentation.md` · Settled 2026-10-09 by the architect: `documentation.md` §5.1's model comment now states the lock-after-path constraint that `D-15` requires, and §9 carries the old wording · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`der-4`)

The model of a working comment is "the child is killed before the stdin writer is joined". `D-4` §Adjudicated found that constraint insufficient, and §Remediated removed the writer thread.

Required property: the document's examples of a working comment state constraints that hold, or are plainly illustrations that do not depend on the tree.

#### F-33 — The folder README's status still says phase 5 is next

2026-10-09 · Tier C · Routed to the record owner · Settled 2026-10-09 by the architect: the folder README's status names the rulings on round `host-initial-refactoring-r2` and phase 6 as next · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`integrity-5`)

The Status line of [`host/initial-refactoring/README.md`](host/initial-refactoring/README.md) says "Next: phase 5 of the plan". The journal and the register record phase 5 as done.

Required property: the folder README's status names the state the journal and the register record.

#### F-34 — `set_x11_text` takes an owned `Vec<u8>` that nothing consumes

2026-10-09 · Tier C · Routed to the implementer · 2026-10-09: `D-10` fixes the seam as a closure, not its parameter's type, so borrowing is owed and decides no contract · Settled 2026-10-09: `clipboard::set_x11_text` and the `set_x11_text` parameter of `sync::sync_text` take `&[u8]` · Remediation in [`plan.md`](host/initial-refactoring/plan.md) §Phase 6 · Review [`host-initial-refactoring-r2`](reviews/host-initial-refactoring-r2/host-initial-refactoring-r2-summary.md) (`cleanup-1`)

`capture` now borrows its input. `clipboard::set_x11_text`, and the seam in `sync.rs` that calls it, still take ownership.

Required property: each parameter's type states what the function does with the value.

#### F-35 — An argument that is not UTF-8 makes the binary panic

2026-10-09 · Tier A · Found by the architect while planning `D-16` · Ruled 2026-10-09 by `D-16` · Settled 2026-10-09 in `D-16` §Implemented: arguments are read as OS strings, and the non-UTF-8 rows of `D-12`'s tables exit 1 · Remediation in [`plan.md`](packaging/plan.md) §Stage 1

`main` collects `env::args()`, which panics on an argument that is not valid Unicode. On branch `host/initial-refactoring` as read on 2026-10-09, with `D-15` implemented, the debug binary was run as `claude-clipboard-host serve $'\xff'` and as `claude-clipboard-host $'\xff'`. Each printed ``called `Result::unwrap()` on an `Err` value: "\xFF"`` from `std/src/env.rs` and exited 101. `D-12` promises exit status 1 and one line, for the usage refusal and for a malformed `serve` alike.

The consequence is an error path that differs at runtime, so the tier is A by `review-findings.md` §Tier. Its reach is small, because only an operator's malformed invocation meets it.

Required property: no argument, whatever its bytes, makes the binary panic, and a refusal of one follows `D-12` (`D-16`).

---

## Questions

#### Q-1 — What should the bridge do when the text-sync watcher exits while it is serving?

2026-10-06 · Open · Owner's decision; does not block the refactoring

If `wl-paste --watch` exits — for example when the compositor restarts — `serve` does not notice. Text sync stops without a word, and image serving goes on.

The options:

- **Status quo**: nothing is reported.
- **Report**: one stderr line naming the tool and its exit status, and serving continues.
- **Restart** the watcher with backoff.
- **Exit**, which also stops image serving.

Recommendation: report and continue. That costs one `try_wait` on an event the accept loop already wakes for, if the watcher's pidfd joins the poll set from `D-7`. Restarting is a supervisor's job, and exiting trades a working feature for a broken one.

#### Q-2 — Under what license are the packages distributed?

2026-10-09 · Answered 2026-10-09 by the owner: Apache-2.0 · Owner's decision; does not block `D-18` unless a format refuses to build without a license value

The repository has no license file, and neither manifest declares one. A `.deb` and an `.rpm` normally carry a license field, and a package that is redistributed needs one. `D-18` ships no license value until the owner chooses one. A license is the owner's grant, and no role can infer it.

Once it is chosen: add the license file, set `license` in the workspace manifest, and give the packaging manifest the same value. A `.deb` carries it in `/usr/share/doc/claude-clipboard/copyright`, and an `.rpm` in its `License` tag.

##### Q-2 §Answer

2026-10-09 · The owner chose the Apache License 2.0.

- `LICENSE` at the repository root is the Apache License 2.0 text as published at `https://www.apache.org/licenses/LICENSE-2.0.txt`, unmodified. Its appendix's copyright line is left as the template gives it.
- `[workspace.package]` sets `license = "Apache-2.0"`, and `crates/host` inherits it.
- `NOTICE` at the repository root carries the copyright line, `Copyright 2026 Vladimir Rindevich`, which the owner gave. `LICENSE` keeps the published text, appendix included, because the appendix is a template for notices and not part of the grant.
- `README.md` §License names the holder and both files.
- Owed by stage 2 of [`plan.md`](packaging/plan.md): the packaging manifest sets the same value, so that the `.rpm`'s `License` tag is `Apache-2.0` and the `.deb` carries the license in `/usr/share/doc/claude-clipboard/copyright`, whose `Copyright:` field is `NOTICE`'s line. `D-18`'s "no license value" property no longer applies, because the condition it waited on is met.

---

## Investigations

#### I-1 — Baseline, and capture throughput

2026-10-06 · Closed · Tree: the initial implementation on branch `host/initial-refactoring` as of this date · Machine: the repository's devcontainer, `rustc 1.98.1`

Baseline:

- `cargo test --workspace`: 13 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- The release binary `target/x86_64-unknown-linux-musl/release/claude-clipboard-host` is **619,264 bytes**.
- Its normal dependency edges are `libc` and `serde_json`, which brings `itoa`, `memchr`, `serde_core` and `zmij`.
- A clean release build for the musl target takes 4.2 s.

Capture throughput: `capture("head", ["-c", N, "/dev/zero"], None, 30 s)`, release profile, run as an ignored test in a scratch copy of the crate:

| Payload | Time    |
| ------- | ------- |
| 1 MiB   | 71 ms   |
| 8 MiB   | 567 ms  |
| 32 MiB  | 2.25 s  |
| 63 MiB  | 4.41 s  |

The same 63 MiB capture under the production 4 s bound returned `Err("head timed out")`. The probe was not retained. Its method is in [`analysis.md`](host/initial-refactoring/analysis.md) §Measurements.

#### I-2 — Operating-system and protocol probes

2026-10-06 · Closed · Same tree and machine as `I-1`

Each probe ran against the release binary, or against a scratch copy of the crate, with stand-in tools on `PATH`. Methods are in [`analysis.md`](host/initial-refactoring/analysis.md) §Measurements.

- **Header writes.** `strace -f` of `serve` answering one read showed 14 `sendto` calls for the header and 1 for a 1,000,008-byte payload. SIGTERM removed the socket and the bridge exited 0.
- **Slow peer.** A peer sent one byte every 10 s. After 30 s its connection was still open and no response had been written.
- **Watcher orphan.** After `serve --sync-text` was killed with SIGKILL, the stand-in `wl-paste --watch` was still alive with parent PID 1.
- **Non-UTF-8 listing.** `respond_with` was given the listing `image/png\ntext/x-\xff\n` and a read of `image/png`. It returned `Err(Utf8Error { valid_up_to: 17, error_len: Some(1) })`.
- **Signal mask inheritance.** With SIGINT and SIGTERM blocked in the parent thread, a child spawned through `Command` reported `SigBlk: 0000000000004002`, the same as the parent. `std` does not reset the signal mask for a child. This rejects `signalfd` in `D-7`.
- **`std` availability on 1.98.1.** `File::try_lock` and `std::io::pipe` are stable. `UnixStream::peer_cred` (`peer_credentials_unix_socket`) and `CommandExt::create_pidfd` (`linux_pidfd`) are not.

#### I-3 — Cost of `serde` derive for the request

2026-10-06 · Closed · Same tree and machine as `I-1`

A scratch copy replaced the `Value`-based request parsing with `#[derive(Deserialize)] #[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)] enum Request { Types, Read { type: Option<String> } }`. It took `serde` with the `derive` feature through `cargo add`.

| Build    | Release binary  | Clean release build |
| -------- | --------------- | ------------------- |
| Baseline | 619,264 bytes   | 4.2 s               |
| Derive   | 647,936 bytes   | 6.6 s               |

The derive build is 28,672 bytes (+4.6%) larger and 2.4 s slower to build clean. It also adds `serde_derive`, `syn`, `quote`, `proc-macro2` and `unicode-ident` to the build. Separately, `should_reject_invalid_requests` failed on `{"op":"types","command":"evil"}`: `deny_unknown_fields` does not apply to the unit variant of an internally tagged enum. This rejects derive in `D-3`.
