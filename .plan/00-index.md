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
- The header's bytes are unchanged: `{"ok":true,"size":N}` or `{"ok":false,"size":0,"error":"…"}` followed by a newline.

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

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-6` · Mechanism chosen by `I-2`

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

#### D-9 — The `handle-stdio` subcommand is deleted

2026-10-06 · Accepted · Implemented 2026-10-06 · Addresses `F-8`

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

2026-10-07 · Tier A · Open · Routed to the architect (`D-7`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-1`)

`sys::wake_on_termination` installs a SIGHUP handler unconditionally and never reads the old action. A disposition inherited as ignored, as under `nohup`, was kept by the initial implementation and is now replaced, so a bridge that used to outlive its terminal shuts down with it.

Required property: a signal the bridge inherited as ignored stays ignored, or the record states that ignoring SIGHUP no longer detaches the bridge.

#### F-11 — After a termination signal the socket stays bound and new connections queue unserved

2026-10-07 · Tier A · Open · Routed to the architect (`D-7`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-2`)

On the wake byte, `serve` joins every worker inside `thread::scope`, and only after that drops `Bridge`, which removes the path and closes the listener. While the workers are joined, the kernel queues new connections that nothing will accept. With one idle connection open, the process exited 11.9 s after SIGTERM, and a client that connected meanwhile got no reply and then a reset.

Required property: once shutdown is requested, a new connection is refused promptly or served, and `D-7` states which ordering "as soon as the signal is delivered" means.

#### F-12 — Running-server coverage is claimed for wiring that no test pins

2026-10-07 · Tier B · Open · Routed to the implementer · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-3`)

`README.md` §Tests and limitations and `test-architecture.md` §The layers claim that tests cover the running server's peer-UID check, its concurrency limit and its per-connection deadline. Two mutations survived the full suite: replacing the `admit` call in `serve` with an unconditional slot, and raising `PHASE` to 3600 s.

Required property: every coverage claim can be falsified by mutating the site it claims, or the text says the property is proved only at the function level.

#### F-13 — A failed capture with input can outlive its deadline

2026-10-07 · Tier C · Open · Routed to the architect (`D-4`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-1`)

When a tool exits non-zero while another member of its group holds stdin open, `capture` joins the stdin writer after dropping a reaped `Tool`. That drop signals nothing, so the join waits for the group member to exit. A probe took 3.01 s against a 200 ms deadline; the initial implementation returned in 2.45 ms. `D-4`'s "one deadline bounds the whole capture" and "a reaped child is never signalled" conflict on this path. No production trigger is known, because `xsel -i` reads all of its input before it forks.

Required property: a capture's total duration, the stdin join included, is bounded on its failure paths.

#### F-14 — `F-6`'s per-connection limb has no test that drives the accept loop

2026-10-07 · Tier C · Open · Routed to the owner or the architect · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-2`)

Plan step 3.5 requires a test showing that a per-connection failure leaves the loop running, and none exists. `admit`'s tests pin refusal messages and slot counting, which were not the defect. The journal acknowledges the gap.

Required property: either a test shows that a per-connection failure leaves `serve` accepting, or the waiver of that exit criterion is recorded with who granted it.

#### F-15 — Three binary tests start `serve` with no path that ends it

2026-10-07 · Tier C · Open · Routed to the implementer · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-3`)

`Scratch::run` waits for the binary to exit, and three tests use it to start `serve`, expecting it to refuse. A regression that lets `serve` start makes `cargo test` and the image build hang rather than fail, which breaks `D-2`'s termination property.

Required property: every binary test that starts `serve` ends it and reports a failure on every outcome.

#### F-16 — `D-6` gives the error header's key order differently from the bytes the host sends

2026-10-07 · Tier C · Open · Routed to the architect (`D-6`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-4`, `integrity-4`)

`D-6` gives `{"ok":false,"size":0,"error":…}`. The host emits sorted keys, `{"error":…,"ok":false,"size":0}`, and always has.

Required property: the record describes the header's wire form as the host emits it, or says that field order is not part of the contract.

#### F-17 — A graceful shutdown kills an in-flight text sync

2026-10-07 · Tier C · Open · Routed to the architect (`D-8`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-5`)

On SIGTERM, SIGINT or SIGHUP, the `Bridge` drop kills the watcher's process group, and every running `sync-text` is in that group. A probe confirmed this. `D-8` and the `clipboard::watch` doc comment say a running sync runs to completion. That holds only on parent death. The behaviour is unchanged from the initial implementation.

Required property: the record and the comment state truthfully what happens to an in-flight sync on each way the bridge ends.

#### F-18 — Malformed `serve` options do not print `D-9`'s usage line

2026-10-07 · Tier C · Open · Routed to the architect (`D-9`, `D-3`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-6`)

`serve --bogus`, `serve --allow-uid alice` and `serve --allow-uid` exit 1 with a specific message, unchanged from the initial implementation as `D-3` requires. `D-9` says "any other invocation prints the usage line".

Required property: the record states one rule for what a malformed `serve` invocation prints, and the binary follows it.

#### F-19 — The journal cites a commit range as provenance

2026-10-07 · Tier C · Open · Routed to the record owner · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`reviewer-7`, `integrity-5`)

The phase-3 entry of [`journal.md`](host/initial-refactoring/journal.md) gives the span in which a defect existed as a range of commit identifiers. `documentation.md` §10 forbids that in a tracked file.

Required property: the record names the affected states in §10's vocabulary, not by commit identifiers.

#### F-20 — `serve` can pass `Deadline` a blocking stream

2026-10-07 · Tier C · Open · Routed to the implementer · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`integrity-6`)

If `set_nonblocking` fails on an accepted stream, `serve` still writes the refusal through `Deadline::new`, whose documented precondition is a non-blocking stream.

Required property: any path that hands `Deadline` a stream meets its precondition, or the call site states the assumption it relies on.

#### F-21 — `SA_RESTART` and the `accept` loop's `Interrupted` retry guard against a failure that cannot occur

2026-10-07 · Tier C · Open · Routed to the architect (`D-7`, `F-6`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`der-1`)

The listener has always been non-blocking, and a non-blocking `accept` never sleeps, so a signal cannot interrupt it with `EINTR`. A probe saw 0 `EINTR` in 200,000 non-blocking `accept` calls across 7.4 million handler runs; the blocking control returned `EINTR` on all 2,000 calls. `F-6`'s third bullet, `D-7`'s `EINTR` clause, `SA_RESTART` and its comment all rest on the opposite premise.

Required property: every mechanism in the signal and accept path, and the comment that justifies it, describes a failure the serving process can actually meet.

#### F-22 — `Deadline`'s doc comment restates a withdrawn premise

2026-10-07 · Tier C · Open · Routed to the implementer · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`der-2`)

The comment begins "A socket timeout bounds a single call", the premise `D-6` §Adjudicated withdrew.

Required property: the stated reason for `Deadline` matches the adjudicated premise.

#### F-23 — The error alias carries `Send + Sync` that no caller needs

2026-10-07 · Tier C · Open · Routed to the implementer · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`cleanup-1`)

No error typed with `crate::Result` crosses a thread, and the crate builds with the bounds removed.

Required property: the error type's bounds are no wider than some caller needs.

#### F-24 — The `handle-stdio` refusal test repeats the usage-refusal test

2026-10-07 · Tier C · Open · Routed to the architect (`D-9`) · Review [`host-initial-refactoring-r1`](reviews/host-initial-refactoring-r1/host-initial-refactoring-r1-summary.md) (`cleanup-2`)

`should_refuse_handle_stdio_as_an_unpublished_invocation` asserts the same behaviour as `should_print_usage_and_exit_1_for_an_unpublished_invocation`, and its clipboard stand-in plays no part in its assertion. `D-9` §Implemented and plan step 7 name it as `D-9`'s witness, so whether it should stay is a question about `D-9`.

Required property: the refusal of unknown invocations is pinned where the record requires it, and no test carries setup its assertion does not depend on.

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
