# Analysis of the host as first implemented

What `crates/host` does today, which of its properties are load-bearing, and where it falls short. The tree described is the initial implementation on branch `host/initial-refactoring` as it stood on 2026-10-06. Code sites are named by function in `crates/host/src/main.rs`.

The entries this document supports are canonical in [`00-index.md`](../../00-index.md): findings `F-1`–`F-9`, investigations `I-1`–`I-3`.

## What the binary is

One file, 572 lines, three subcommands:

| Subcommand     | Entry       | Purpose                                                                               |
| -------------- | ----------- | ------------------------------------------------------------------------------------- |
| `serve`        | `serve`     | Bind the Unix socket, answer image requests, optionally run the text-sync watcher      |
| `sync-text`    | `sync_text` | Invoked by `wl-paste --watch` for each clipboard change; mirrors plain text into X11   |
| `handle-stdio` | `main`      | Answers one request from stdin to stdout; unpublished and unused (`F-8`)               |

The request path: `serve` accepts a connection, checks the peer UID, and reserves one of 16 slots. It then spawns a thread, which runs `process_request` to read the request line, then `respond_with` to parse the request and query the clipboard through `clipboard` → `capture` → `wl-paste`, then `write_response` to frame the result.

## Properties worth keeping

The refactoring changes how these hold, never whether they hold. Each one is either required by `CONTRIBUTING.md` §Priorities or is the only thing standing between the container and something it must not get.

**Protocol and policy**

- A request is one line of at most 4096 bytes including its newline. A longer or unterminated line is refused.
- Request keys are within `op` and `type`, and `op` is `types` or `read`.
- For `read`, `type` absent, `null` and `"image"` all mean the first offered image. A concrete type must be both offered and supported.
- Supported formats in preference order: `image/png`, `image/jpeg`, `image/jpg`, `image/gif`, `image/webp`. The `types` response lists the offered ones in that order, once each, newline-terminated, and is empty when there are none.
- One listing and one read per request.
- After the read, the bytes must start with the format's magic. This is the guard against the clipboard changing between listing and reading.
- No payload over 64 MiB.
- Response framing: a JSON header line `{"ok":true,"size":N}` or `{"error":"…","ok":false,"size":0}`, then exactly `N` payload bytes.

  > 2026-10-08 — The error header used to be spelled `{"ok":false,"size":0,"error":"…"}` here, in an order the host never sent (`F-16`).

**Access**

- The socket is mode 0666, and every connection's peer UID is checked against the set {serving UID, 0} ∪ `--allow-uid`. A refusal names the UID and the flag to add.
- At most 16 concurrent workers. The excess is refused with a message.

**State the bridge creates**

- The lock file is `<socket path with extension .lock>`, mode 0600, held with a non-blocking exclusive `flock` for the life of `serve`. A second instance fails.
- An existing path at the socket location is replaced only if it is a socket owned by the serving UID. Anything else is refused.
- On a clean exit the watcher is killed, then the socket path is removed, then the listener closes, then the lock is released. Removing the path before releasing the lock is what stops the exiting instance from deleting a socket that a newly started instance has just bound at the same path. Today this order holds only by the declaration order of locals in `serve`.

**Tools**

- Every tool runs in its own process group, with stderr discarded.
- On failure or timeout the group is killed and the child reaped.
- On success the group is left alone. `xsel -i` forks a daemon that owns the X11 selection, and it must survive both the capture and the bridge's shutdown, which kills the watcher's group, not xsel's.

**Text sync** (`sync_text`, `may_sync_text`)

- Drain the watcher's stdin first, so `wl-paste --watch` never blocks writing to it.
- Skip unless `CLIPBOARD_STATE` is unset, `data` or `sensitive`.
- Skip offers containing any `image/` type, including mixed offers. Require some `text/plain…` type.
- Read the text. If X11 already holds the same bytes, stop. **This is what breaks the feedback loop**: writing X11 makes XWayland update the Wayland clipboard, which fires the watcher again.
- List and filter again immediately before writing, which narrows the window in which a freshly copied image could be overwritten by text. The window cannot be closed from outside the compositor.

**Exit**

- Errors reach the user as `claude-clipboard-host: <message>` on stderr with status 1, or as the response header's `error`, which the shim prints.

## Findings in detail

### Capture throughput (F-1)

Canonical entry: `F-1` — capture is capped near 14 MiB/s.

`capture` sets stdout non-blocking. It then loops: try one read of up to 16 KiB, try `try_wait`, sleep 1 ms. The loop sleeps even when data was just read, so each iteration moves at most 16 KiB and takes at least 1 ms, which makes the theoretical ceiling about 16 MiB/s. 14.3 MiB/s was measured (`I-1`). The tool cannot fill the 64 KiB pipe faster than that drains it, so the producer stalls.

The defect hides because every test payload is 22 bytes. A screenshot is typically 1–10 MiB, so the cost in normal use is tens to hundreds of milliseconds of added paste latency. That is felt, not seen.

### Unbounded connection phases (F-2)

Canonical entry: `F-2` — a slow peer holds a worker slot without bound.

`set_read_timeout(12 s)` applies to each `read` syscall, not to `read_until` as a whole. `BufReader::read_until` issues a new read every time its buffer runs dry, and a trickling peer keeps it running dry. The write side has the same structure: `write_all` of a 64 MiB payload is many `sendto` calls, each with a fresh 12 s budget. The peer is the end user in the sense of `CONTRIBUTING.md` §Litmus test — its input is inside the domain by definition — so this is a containment defect, not hardening.

### Watcher lifetime (F-3)

Canonical entry: `F-3` — the watcher outlives a killed bridge.

The watcher is a `ChildGuard` local in `serve`, so it is killed only when `serve`'s frame unwinds. SIGKILL, the OOM killer and the default action of SIGHUP do not unwind. The orphan is functional, not inert: it keeps running `sync-text` on every copy. The next `serve` acquires the lock without trouble, because `flock` died with the old process, and adds a second watcher.

### Listing encoding (F-4)

Canonical entry: `F-4` — a non-UTF-8 listing entry fails every image read.

`std::str::from_utf8(&raw_types)?` appears in `respond_with` and twice in `sync_text`. MIME strings come from the clipboard's owner, which can be any application. The supported types are ASCII, so byte comparison loses nothing.

### Header framing (F-5)

Canonical entry: `F-5` — one syscall per JSON token.

`serde_json::to_writer(&mut output, &header)` on an unbuffered `&UnixStream`. serde_json's formatter writes each token with its own `write_all`. Under `handle-stdio` stdout is line-buffered, so the defect does not show there; it shows only on the socket, which is the path that matters.

### Server lifecycle (F-6)

Canonical entry: `F-6` — shutdown on a timer, abandoned workers, fatal per-connection errors.

- **The 1 s tick.** `poll(listener, 1000 ms)` exists because the comment's premise is true: a process-directed signal may be delivered to any thread that does not block it, so the main thread's `poll` is not guaranteed to see `EINTR`. The tick is the workaround. A wake pipe written by the handler removes the need for it, because the pipe is readable whichever thread ran the handler.
- **Abandoned workers.** `serve` returns from its loop and `main` exits the process while worker threads may be inside `capture`. Their `ChildGuard`s never drop. In practice `wl-paste` usually gets `EPIPE` when the process holding the read end disappears and exits on its own, but nothing guarantees it, and `CONTRIBUTING.md` §Priorities lists _no unreaped child_ among the properties nothing trades against.
- **Fatal per-connection errors.** In the accept loop, `stream.set_read_timeout(…)?`, `stream.set_write_timeout(…)?` and `Err(error) => return Err(error.into())` on `accept` all end the server for a problem with one connection. Handlers are installed without `SA_RESTART`, so a signal that lands between a ready `poll` and `accept` makes `accept` fail with `EINTR`: `serve` returns `Err`, cleanup still runs, and the exit status is 1 instead of 0. `thread::spawn` panics if the thread cannot be created.

  > 2026-10-08 — The `EINTR` sentence in this bullet is falsified (`F-21`). The listener is non-blocking, and a non-blocking `accept` never sleeps, so no signal can fail it with `EINTR`. A probe saw none in 200,000 calls. The rest of the bullet stands.

### Structure and types (F-7)

Canonical entry: `F-7`.

| Smell                                                                  | Where                           | Rule it meets                                            |
| ---------------------------------------------------------------------- | ------------------------------- | -------------------------------------------------------- |
| MIME types, operations and tool flags as `&str`                        | `FORMATS`, `respond_with`, `clipboard`, `image_valid` | `CONTRIBUTING.md` §Types (newtype where a value has identity) |
| `ChildGuard(Child, bool)`: "reaped" as a flag                          | `ChildGuard`, `capture`         | §Types (enum over `bool`)                                |
| Closure seam over argument arrays; tests match `["--list-types"]`      | `respond_with`, `tests::mock`   | test-architecture §Test observable contracts             |
| Nine `unsafe` blocks across four functions, no `SAFETY` comments       | `ChildGuard::drop`, `capture`, `peer_uid`, `serve` | §1.2 (types over guards); review cost                    |
| `flock` through `libc` though `std` has `File::try_lock`               | `serve`                         | §Language and platform (prefer `std`)                    |
| `RUNNING` static flag plus a 1 s tick for shutdown                     | `stop`, `serve`                 | §5 (one authoritative representation)                    |
| `Arc<HashSet<u32>>` and `Arc<AtomicUsize>` for state that outlives no scope | `serve`                    | §9 (copies need an ownership reason)                     |
| Production and test code in one file                                   | `main.rs`                       | Readability; the user's request                          |

What is **not** a smell, and stays:

- `Box<dyn Error + Send + Sync>` as the error type. `CONTRIBUTING.md` §Errors chose it, and nothing branches on an error's kind except `io::ErrorKind`.
- `serde_json` itself. The request is untrusted JSON, and a hand-written parser is the error-prone alternative.
- Hand-written argument parsing. Two subcommands and two flags do not pay for `clap`.
- A thread per connection. `CONTRIBUTING.md` §Language and platform excludes `async`.
- The `respond_with` seam. `CONTRIBUTING.md` §2.1 names it as the boundary that makes the protocol testable, and `D-3` changes its parameter, not its existence.

### Dead surface (F-8)

Canonical entry: `F-8`. `grep -rn handle-stdio` over the repository outside `target/` finds only its match arm in `main`.

### Test coverage (F-9)

Canonical entry: `F-9`.

| Area                                         | Tested today                       |
| -------------------------------------------- | ---------------------------------- |
| Request policy, MIME selection, magic check  | yes, `respond_with` with a mock    |
| Framing                                      | yes, into a `Vec` and across a socket pair |
| Capture: timeout, binary-safe output         | yes, with `sh` and `cat`           |
| Peer UID                                     | yes, on a socket pair              |
| Text-sync type filter                        | yes                                |
| Text-sync sequence                           | no                                 |
| Lock exclusion, stale socket replacement, refusal of a foreign path | no          |
| Signal handling and cleanup                  | no                                 |
| Concurrency limit                            | no                                 |
| Payloads above 22 bytes                      | no                                 |
| Argument parsing, usage and exit status      | no                                 |

## Measurements

Methods for `I-1`–`I-3`, so they can be re-run against the refactored tree. All ran in the repository devcontainer with `rustc 1.98.1`. The scratch copies were made with `cp -r Cargo.toml Cargo.lock .rustfmt.toml crates <dir>`, and no probe was committed.

**Capture throughput (`I-1`).** An `#[ignore]`d test appended to a scratch copy's `main.rs`, run with `cargo test --release probe -- --ignored --nocapture --test-threads=1`:

```rust
for mib in [1usize, 8, 32, 63] {
    let n = (mib * 1024 * 1024).to_string();
    let t = Instant::now();
    let r = capture("head", &["-c", &n, "/dev/zero"], None, Duration::from_secs(30));
    eprintln!("capture {mib:>2} MiB: {:?} in {:?}", r.map(|b| b.len()), t.elapsed());
}
```

After `D-4`, the same loop over the new capture function is the comparison.

**Header writes (`I-2`).** Put a stand-in `wl-paste` first on `PATH`. It prints `image/png\ntext/plain\n` for `--list-types`; otherwise it prints the PNG signature followed by 1,000,000 zero bytes. Then:

```sh
PATH=$fake:$PATH CLAUDE_CLIPBOARD_SOCKET=$dir/s.sock \
  strace -f -qq -e trace=write,sendto,sendmsg,writev -o $dir/trace serve &
# connect, send {"op":"read","type":"image/png"}\n, read to EOF, then SIGTERM
```

Then count the `sendto` calls on the connection's file descriptor.

**Slow peer (`I-2`).** Against the same `serve`, a Python client sends `{"op":` one byte every 10 s and checks after 30 s whether the connection is still open with nothing received.

**Watcher orphan (`I-2`).** A stand-in `wl-paste` that, given `--watch`, writes its PID to a file and `exec`s `sleep 300`. Start `serve --sync-text`, wait for the PID file, `kill -KILL` the bridge, then `kill -0` the watcher's PID.

**Signal mask (`I-2`).** In a test: block SIGINT and SIGTERM with `pthread_sigmask`, then `capture("grep", ["SigBlk", "/proc/self/status"])` and compare it with the thread's own `SigBlk`.

**Derive cost (`I-3`).** In a scratch copy, run `cargo add serde --features derive`, replace the `Value` parsing with the derived enum, `cargo clean`, and time `cargo build --release --target x86_64-unknown-linux-musl` from Python's `time.monotonic()`. Do the same on an untouched copy.
