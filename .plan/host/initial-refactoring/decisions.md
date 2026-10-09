# Decisions: the alternatives and why they lost

The decisions themselves — statement, argument and required properties — are canonical in [`00-index.md`](../../00-index.md) as `D-1`–`D-10`. This document keeps what the register entries do not: the options that were weighed, and why each one lost. An implementer owes the required properties in the register, and nothing here.

### Module layout and `unsafe` confinement (D-1)

Canonical entry: `D-1` — modules by responsibility, `unsafe` in one module.

- **Keep one file and reorder it.** Rejected. Ordering does not stop one responsibility's helpers from being used by another, and it does nothing for the tests, which would stay in the same file.
- **A library crate plus a thin binary**, with `src/lib.rs` exposing the modules so that `tests/` could call them. Rejected. It creates a public surface solely for tests, which `CONTRIBUTING.md` §4 and `test-architecture.md` §The layers both refuse. It also turns every internal function into a semver question for a crate nobody depends on.
- **A workspace of several crates**, for example protocol and server. Rejected. There is no second consumer, and every crate boundary costs compile time and a manifest.
- **Leave `unsafe` at its call sites but add `SAFETY` comments.** Rejected as insufficient. The comments would be correct, but nothing would stop the next site from appearing anywhere. Denying `unsafe_code` everywhere except one module makes the boundary something the compiler enforces.
- **Replace `libc` with `rustix` or `nix`.** Rejected. Each is a larger dependency than `libc` for about six calls, and `CONTRIBUTING.md` §Language and platform names `libc` as one of the two dependencies the host carries. Revisit only if the `sys` module grows past what one reviewer can hold.

### Test layout (D-2)

Canonical entry: `D-2` — sibling test files and a black-box binary layer.

The request was for "separate tests". There were three readings:

- **Sibling files** (`foo.rs` + `foo/tests.rs`, declared `#[cfg(test)] mod tests;`). Taken. It is the standard idiom, keeps crate-private access, and publishes nothing.
- **Move everything to `crates/host/tests/`.** Rejected for the unit layer. Integration tests reach only what the crate publishes, and a binary publishes nothing to Rust. Every unit test would need either a library target (rejected above) or a route through the binary, and the binary cannot reach pure policy with a mock clipboard.
- **Both, divided by what each must reach.** Taken. Unit and OS-boundary tests stay beside their module in sibling files. A new layer under `tests/` reaches what only the running binary can show: socket lifecycle, lock, signals, deadlines, the watcher.

Choices inside the binary layer:

- **Stand-in tools as `/bin/sh` scripts on `PATH`**, against a hidden environment variable that points the binary at mock commands. The variable is rejected. It would be a published value that exists only for tests, which §4 rejects by name. `PATH` is already how the binary finds `wl-paste` and `xsel`.
- **`tempfile` as a dev-dependency** against a hand-rolled directory under `std::env::temp_dir()`. Hand-rolled is taken, because a few lines of test support do not justify a dependency. `CARGO_TARGET_TMPDIR` was considered and rejected: it sits under the checkout, so a deep checkout path could push a socket path past the 107-byte `sun_path` limit.
- **Real `wl-paste` in the default run.** Rejected, as `test-architecture.md` already decides: a fresh checkout has no Wayland session. The real-tool check stays manual or `#[ignore]`d.

### Domain types (D-3)

Canonical entry: `D-3` — formats, requests, queries and commands are types.

- **`serde` derive for `Request`.** Measured and rejected (`I-3`): +28,672 bytes, +2.4 s of clean build, five more build-time crates, and a `deny_unknown_fields` gap on internally tagged unit variants that an existing test catches. Struct variants (`Types {}`) would close the gap, but not the cost.
- **Drop `serde_json` and parse the request by hand.** Rejected. The request is untrusted, and a hand-written JSON parser is the most error-prone code this crate could contain. The response header could be formatted by hand, but the error string still needs JSON escaping, and `serde_json` is in the build for the request anyway.
- **A `MimeType(String)` newtype** against an enum of formats. Rejected. The set is closed and each member has behaviour of its own (magic bytes), which is what an enum is for. A string newtype would still admit `image/bmp`.
- **A trait `Clipboard` with a Wayland implementation and a test implementation.** Rejected per `CONTRIBUTING.md` §10. There is one production implementor, and a closure over a query enum is the lighter seam that does the same job.
- **`thiserror` or a hand-written error enum.** Rejected per `CONTRIBUTING.md` §Errors. Every error ends as text, and nothing matches on its kind.
- **`clap` for arguments.** Rejected. Its compile time and binary size are out of all proportion to two subcommands and two flags.

### Capture mechanism (D-4)

Canonical entry: `D-4` — event-driven reading under one deadline, with a guard that always reaps.

- **Keep the loop, shorten or remove the sleep.** Rejected. Without the sleep it is a busy loop at 100% CPU for the duration of every capture; with a shorter sleep it is the same defect at a different ceiling.
- **A reader thread with blocking `read_to_end`, and the caller waiting on a channel with `recv_timeout`.** Considered seriously: pure `std` and no `unsafe`. It loses on the exit side. Once stdout reaches EOF, the child must still be waited for under the deadline, and `std` has no wait with a timeout. That brings back either a `try_wait` poll loop or a second thread per capture.
- **`poll` over the stdout pipe and a pidfd for the child.** Taken. One thread, no timer, both events in one wait, and the deadline is the `poll` timeout. The cost is `pidfd_open` through `libc::syscall` (Linux 5.3 and later; Fedora's kernel qualifies) and two `unsafe` calls in the `sys` module. `std`'s own pidfd support (`linux_pidfd`) is unstable (`I-2`) and would replace the syscall when it stabilises.
- **`SIGCHLD` handling.** Rejected. It is process-wide state, and it interacts with every other child the process has — the watcher, other workers' tools.
- **Enlarge the pipe with `F_SETPIPE_SZ`.** Not decided. Once the sleep is gone the producer is the bottleneck, and whether a larger pipe helps it is a measurement for after `D-4`. Any change goes through `CONTRIBUTING.md` §17 as a reported optimisation with its delta.

### Listing as bytes (D-5)

Canonical entry: `D-5`.

- **`String::from_utf8_lossy` over the listing.** Rejected. It allocates a second copy of the listing to compare ASCII strings that byte comparison handles directly.
- **Per-line UTF-8 validation, skipping invalid lines.** Equivalent in outcome, but it does work that changes nothing: a supported type is ASCII, so a line that fails UTF-8 validation also fails byte equality.

### Connection deadlines and framing (D-6)

Canonical entry: `D-6`.

- **A watchdog thread per connection that shuts the socket down at the deadline.** Rejected. It doubles the thread count to enforce what a deadline enforces in the worker's own thread.
- **Non-blocking sockets with one `poll` loop for all connections.** Rejected. It is an event loop, and `CONTRIBUTING.md` §Language and platform keeps a thread per connection. A non-blocking stream per worker, where each worker polls only its own stream, is a different thing. That is what was built (`D-6` §Adjudicated).
- **Reset the socket timeout to the remaining time before each call.** This was the mechanism the decision first assumed. It fails, because the kernel takes `SO_SNDTIMEO` afresh for each buffer it allocates within one call. A single large write to a slow reader therefore runs past any per-call timeout: 6.3 s against 200 ms in the measurement `D-6` §Adjudicated records.

> 2026-10-07 — The watchdog bullet used to end: _"It doubles the thread count to enforce what resetting the socket timeout to the remaining time before each call enforces in the same thread."_ Its premise was falsified while `D-6` was being implemented. The rejection stands, because the deadline still runs in the worker's own thread.
- **`BufWriter` around the stream.** Rejected for the payload. `BufWriter` passes large writes through unbuffered, so it would work, but it only hides the header's 14 small writes in its buffer, where a deliberately framed header states the intent. Either form satisfies the required property. The decision is the property, not the spelling.
- **A single `Vec` holding header and payload.** Rejected outright. It copies up to 64 MiB to save one syscall, which `CONTRIBUTING.md` §0 and §9 forbid.

### Server lifecycle (D-7)

Canonical entry: `D-7`.

- **`signalfd`.** Rejected on evidence (`I-2`). It requires the signals blocked in every thread, and spawned tools inherit the blocked mask. Unblocking them in each child needs `pre_exec`, which is `unsafe` and forces `fork` over `posix_spawn` for every tool on the request path.
- **Keep the flag and shorten the tick.** Rejected. It trades idle wake-ups against shutdown latency, and the wake pipe removes both.
- **Exit without joining workers**, relying on `wl-paste` getting `EPIPE`. Rejected. That is the abandoned-child defect `F-6` names.
- **Join workers, but interrupt them at shutdown by calling `shutdown(2)` on their sockets.** Deferred. It would cut the worst-case shutdown from roughly 32 s (12 s read + 8 s of tools + 12 s write) to the tools' 8 s, at the cost of tracking live streams. A slow peer that sits through both deadlines during a shutdown is unlikely, and SIGKILL remains for the impatient now that `D-8` covers the watcher. Reopen if a measured shutdown is ever a complaint.
- **SIGHUP.** Added to the handled set. A terminal that closes sends it, and README step 2 runs the bridge in a terminal. Handling it costs one more `sigaction`, and the bridge has no configuration to reload, so the conventional reload meaning does not apply.

### Watcher lifetime (D-8)

Canonical entry: `D-8`.

- **`PR_SET_PDEATHSIG` in `pre_exec`.** Taken. It is the kernel mechanism made for this. `pre_exec` is `unsafe` and forces `fork`, but the watcher is spawned once per `serve`, never on the request path.
- **Run the watcher under the bridge's own process group,** so that a terminal's signal reaches it too. Rejected. It covers terminal signals only, not SIGKILL or the OOM killer.
- **A subreaper (`PR_SET_CHILD_SUBREAPER`).** Rejected. It keeps orphaned descendants for a living bridge to reap; it does nothing once the bridge itself is dead.
- **SIGTERM as the death signal** against SIGKILL. SIGKILL is taken. `wl-paste --watch` has no cleanup to run, and the guarantee is the point.

### `handle-stdio` (D-9)

Canonical entry: `D-9`.

- **Keep it as a test entry point.** Rejected. `CONTRIBUTING.md` §4 says a test seam is not a reason to publish a value, and the binary layer of `D-2` reaches the protocol through `serve`.
- **Document it** for inetd-style or systemd socket activation. Rejected. Nobody asked for it. If it is ever wanted, it should be designed with peer checking.

### Text-sync seam (D-10)

Canonical entry: `D-10`.

- **Test only through the binary layer**, with stand-in `wl-paste` and `xsel`. Rejected as the only layer. Each case would cost several process spawns and file-based state in the stand-ins, where a unit test with closures pins the same sequence in microseconds. One binary-level test that `sync-text` runs end to end against stand-ins is still worthwhile, and the plan includes it.

### Server lifecycle, corrected (D-11)

Canonical entry: `D-11`, which supersedes `D-7`. `D-7`'s alternatives above still stand, apart from the deferral that the last bullet of this section reopens.

**Inherited dispositions (`F-10`):**

- **Honour an inherited `SIG_IGN` for SIGHUP only.** Rejected. The reason for honouring it applies to every signal: the launcher chose. `nohup` ignores SIGHUP, and a shell that starts a background job without job control ignores SIGINT. One rule for all three costs nothing extra.
- **Keep overriding, and document that `nohup` no longer detaches the bridge.** Rejected. It turns a deliberate operator choice into a trap, and it buys nothing: a bridge that ignores SIGHUP never needs cleanup on SIGHUP.

**Shutdown ordering (`F-11`):**

- **Keep accepting during the drain, and refuse each new connection with `bridge is shutting down`.** Rejected. The message is friendlier than a reset or a missing path, but the loop would have to run alongside the join, watching both the listener and the workers finishing. Removing the path and closing the listener refuses new connections promptly and needs no machinery.
- **Keep the socket until the drain ends, as `D-7` had it.** Rejected on `F-11`'s evidence. Connections queue that nothing accepts, and the socket stays advertised for the whole drain.
- **Cut the response write at shutdown as well.** Rejected. A large response always waits for socket space, so even a reader at normal speed would lose an image already being delivered. The response phase keeps its own 12 s bound.
- **Exit at once on a second signal.** Rejected. It is a second shutdown path, and it abandons the in-flight tools that `D-7` was written to reap. Once the drain covers only work in progress, it is normally milliseconds long.
- **Interrupt workers with `shutdown(2)` on their sockets.** `D-7` deferred this because it needed a registry of live streams. That reason is gone for the request phase: `server::Deadline` already waits in `poll`, and the wake pipe stays readable after the first signal. Watching the pipe there ends the request phase with no registry. This is what `D-11` adopts, for the request phase only.

**`EINTR` (`F-21`):**

- **Keep `SA_RESTART` and the `Interrupted` arm on `accept` as defence in depth.** Rejected. Neither answers a failure the serving process can meet (`F-21`'s probe), and each comes with a reason that is false and would mislead the next reader. A blocking call added later that `std` does not retry would need its own handling in any case, because `SA_RESTART` does not restart every call.

### Capture stdin (D-4, `F-13`)

`D-4` stands. It is adjudicated in `D-4` §Adjudicated.

- **Signal the group of a reaped tool on failure.** Rejected. Once the group's last member exits, its ID may name another group, and `D-4` forbids signalling it for exactly that reason.
- **Detach the writer thread on failure.** Rejected. The thread would outlive the capture.
- **Write stdin without blocking, in the capture's `poll` loop.** Taken. One loop and one deadline cover all three descriptors, and one thread per capture with input goes away.

### In-flight sync on a graceful shutdown (D-8, `F-17`)

`D-8` stands, and its last property is read as scoped to the death of the serving process, in `D-8` §Adjudicated.

- **Stop only the watcher on a graceful shutdown, so that a running sync finishes.** Rejected. Its syncs would be left orphaned, writing to X11 for up to about 17 s after the bridge reported that it had stopped. It would also take a second kill rule for one `Tool`, against `D-4`'s by-group rule. What it prevents is a truncated X11 selection, which needs more than a pipe buffer of text in flight at the moment of shutdown. The next copy repairs it.

### Invocations (D-12)

Canonical entry: `D-12`, which supersedes `D-9`.

- **Print the usage line for malformed `serve` options too**, as `D-9` was worded. Rejected. It discards the one line that says which argument was wrong. It would also have broken `D-3`'s unchanged-text property, which the implementation honoured.
- **Delete the `handle-stdio` witness.** Rejected. It is the regression test for the deletion, and as a row in the usage table it costs one string.

### Commit identifiers in the record (D-13)

Canonical entry: `D-13`.

- **Leave the identifier, because the record is append-only.** Rejected. The violation stays, and the token still resolves to nothing once the branch is squashed.
- **Withdraw it under §6, carrying the old sentence in a note.** Rejected. The note would carry the identifier, so the violation moves rather than goes.
- **Rewrite the sentence with no note.** Rejected. A record edited without a note cannot be told apart from one that was always so. The note records that an edit was made.

### Refusal delivery (D-14)

Canonical entry: `D-14`.

- **The host reads the request before it refuses.** Rejected. The refusals happen on the accept thread, so a peer that never sends would hold every other connection behind it for up to a request phase. `CONTRIBUTING.md` §1.1 puts the peer outside the trust boundary.
- **The host hands each refusal to a thread, or to a timed queue that reads the request and then closes.** Rejected. A refusal would then cost what a worker costs, so the concurrency limit would stop bounding the bridge's work. A queue is new machinery on the accept loop, a timer included, for something the client can do in its own read.
- **The host shuts down its write side and keeps the stream open until the peer has sent.** Rejected for the same reason. The host would hold a descriptor for as long as the peer chose, unless a timer bounded it.
- **Both sides change.** Rejected. The shim's change alone took the loss from 62 of 150 refusals to none under load, and nothing a host change could add was left to buy.
- **Record that a refusal may surface as a broken connection.** Rejected. The refused-UID message is the one `README.md` §Access errors sends a user to for the `--allow-uid` remedy. Losing it is Tier A, and the remedy is a few lines in the shim.

### Server lifecycle, corrected again (D-15)

Canonical entry: `D-15`, which supersedes `D-11`. `D-11`'s alternatives above still stand.

**The `accept` retries (`F-29`):**

- **Keep the `WouldBlock` arm and correct its reason.** This is the least the finding asks for. Rejected. A clean exit on a signal would still depend on one call falling through to another call's error. The retry belongs where the interruption happens, which is `poll`.
- **Keep `ConnectionAborted` as defence in depth.** Rejected, as `SA_RESTART` was (`F-21`). No probe has produced it on `AF_UNIX`, and the reason it would carry is false.
- **Retry every `accept` error.** Rejected. The listener stays readable, so an error that persists, such as running out of descriptors, would make the loop spin instead of ending `serve` with the error.

**Inherited dispositions in the binary layer (`F-26`):**

- **Check the runner's dispositions in `/proc/self/status` and fail with a message that names them.** Rejected as the remedy, although it satisfies the finding's second limb. The property would go unwitnessed in exactly the environment that lacks it, and a precondition a test can establish should be established rather than reported.
- **Skip the signal tests when a disposition was inherited as ignored.** Rejected. The coverage would disappear silently, possibly in the image build, which nobody watches run.
- **Reset the dispositions with `sigaction` in the test support.** Rejected. `unsafe` is confined to `sys` (`D-1`), and the lint table denies it everywhere else.
- **Start `serve` through `timeout`, which happens to reset them.** Rejected. It does so as a side effect, so a reader cannot see why it is there. `env --default-signal` says what it is for.
- **A helper in Python or a second binary.** Rejected. `D-2` provisions `/bin/sh` and coreutils only.

**The ordering witnesses (`F-27`):**

- **Record the two orderings as held by construction only.** Rejected. `F-11` was a Tier A defect in exactly this ordering. Both mutations pass today, and a witness with no timing race exists: a request whose tool the test holds keeps the drain open for as long as the test needs.
