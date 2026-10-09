# Images in Claude CLI inside a devcontainer

The host side is written in Rust. On request over a Unix socket, it reads the
current image from the Fedora Wayland clipboard. Inside the container, the
`wl-paste` and `xclip` commands are replaced by small Python wrappers.

| Where                  | What it needs                                                        |
| ---------------------- | -------------------------------------------------------------------- |
| Fedora x86_64          | The built binary, the real `wl-paste`; `xsel` for text sync          |
| Working devcontainer   | An existing Python 3, the shared directory, a PATH setting           |
| Build environment      | Rust 1.89+ with the musl target, or Docker with the provided Dockerfile |

Python, Cargo and rustc do not need to be installed on Fedora. The Python file
lives in the shared directory but runs only inside the devcontainer. The binary
is built for `x86_64-unknown-linux-musl` and statically linked, so it does not
depend on Fedora's glibc version.

PNG, JPEG, GIF and WebP are supported when the corresponding MIME type is
offered by the clipboard. There is no conversion from other formats. Images are
passed in memory, and no clipboard history is kept. The wrappers export images
only and are not a full replacement for the real clipboard tools. A single
response is limited to 64 MiB.

## 1. Build

All build commands run from the repository root, in a container or anywhere
else with the tools; nothing Rust-related is needed on Fedora.

With a Docker CLI that can reach a Docker daemon:

```bash
docker build -f crates/host/Dockerfile --output type=local,dest=dist .
```

The Rust toolchain and the musl target are installed inside the build image. The
build runs the Rust tests first, then builds the release binary and exports it
as `dist/claude-clipboard-host`. If the tests fail, nothing is exported. The
build container does not need Wayland access.

Alternatively, build in an existing Rust environment, such as this repository's
devcontainer. Only the musl target needs adding:

```bash
rustup target add x86_64-unknown-linux-musl
cargo test --target x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

The result is `target/x86_64-unknown-linux-musl/release/claude-clipboard-host`.
Copy it to `dist/claude-clipboard-host`, or pass its path to `setup-host.sh` in
the next step. Either way it has to be somewhere Fedora can read.

## 2. Install on Fedora

From the repository checkout on Fedora:

```bash
bash setup-host.sh
```

A different path to the binary can be passed as the first argument:

```bash
bash setup-host.sh /path/to/claude-clipboard-host
```

This installs the binary, the container wrapper and the `bin/wl-paste` and
`bin/xclip` links into `~/.local/share/claude-clipboard/`. Do not add that `bin`
directory to the host's PATH.

If a text-sync watcher such as `wl-paste --watch ...` is already running, stop
it: `--sync-text` below replaces it. Then run in a Fedora terminal:

```bash
"$HOME/.local/share/claude-clipboard/claude-clipboard-host" serve --sync-text
```

Leave the terminal open for now; closing it, Ctrl+C or SIGTERM stops the bridge.
It removes its socket at once and stops its text watcher, answers the requests
it has already received, and tells a client still sending one that the bridge is
shutting down. Any of SIGHUP, SIGINT and SIGTERM that the bridge was started
with ignored stays ignored, so under `nohup`, which ignores SIGHUP, it keeps
running when its terminal closes. `--sync-text` copies plain text into X11 with
`xsel`. It checks the current content first and skips offers that contain an
image, including mixed offers where an image comes with a text representation.
If you do not need this, run `serve` without `--sync-text`.

## 3. Mount the shared directory

Mount the whole directory, not just the socket file: that way the server can be
restarted without recreating the containers.

| Side      | Path                                  |
| --------- | ------------------------------------- |
| Fedora    | `$HOME/.local/share/claude-clipboard` |
| Container | `/opt/host-clipboard`                 |

A read-only mount is enough: the container reads the code and connects to the
socket, and creates no files on the host.

Docker Compose, inside the relevant service:

```yaml
volumes:
  - ${HOME}/.local/share/claude-clipboard:/opt/host-clipboard:ro,z
```

For a devcontainer using `image` or `build`, extend the existing `runArgs` and
keep the other arguments:

```jsonc
"runArgs": [
  "--volume",
  "${localEnv:HOME}/.local/share/claude-clipboard:/opt/host-clipboard:ro,z"
]
```

`z` applies a shared SELinux label, usable by several containers. Do not use `Z`
for this shared directory. Docker `--mount` does not support relabelling, which
is why the example uses `--volume`. Recreate the container after changing its
mounts.

## 4. PATH in the container

In a new container terminal:

```bash
export PATH="/opt/host-clipboard/bin:$PATH"
command -v wl-paste xclip
```

Expect `/opt/host-clipboard/bin/wl-paste` and `/opt/host-clipboard/bin/xclip`.
To make it permanent, add the export to the container user's shell
configuration, or set PATH through the image, Compose or devcontainer settings.

The socket path is derived from the mounted directory. It can be overridden with
`CLAUDE_CLIPBOARD_SOCKET` on both sides. There is no need to set a dummy
`DISPLAY` or `WAYLAND_DISPLAY`.

## 5. Check pasting

On Fedora, copy a screenshot as an image. In the container:

```bash
wl-paste --list-types
wl-paste --type image/png | wc -c
```

For a PNG, the first command should list `image/png` and the second should print
a non-zero size. For a JPEG, use `image/jpeg`. Then start Claude from the same
terminal, or resume a session, and press Ctrl+V. An image attachment should
appear. A Claude session that is already running does not see the new PATH, so
restart or resume it.

If reading works but Claude does not paste the image, enable tracing before
starting it:

```bash
export CLAUDE_CLIPBOARD_TRACE=/tmp/claude-clipboard-trace.log
```

Paste again and check the file. The log contains only tool names and arguments,
never clipboard content, and shows how a given CLI version calls the tools. If
the file does not appear, the terminal may have intercepted Ctrl+V, or the CLI
reads the clipboard some other way.

## 6. Autostart in niri

Once the check passes, add this at the top level of `config.kdl`:

```kdl
spawn-at-startup "sh" "-c" "exec \"$HOME/.local/share/claude-clipboard/claude-clipboard-host\" serve --sync-text"
```

It takes effect at the next niri start; saving the config does not rerun startup
commands in the current session. For the current session, use the process from
step 2. Do not run a separate text watcher alongside it.

## Access errors

By default the server accepts connections from the Fedora user's UID and from
root. Other UIDs are refused. If the container runs as a different UID, or uses
a user-namespace mapping, the error names the UID the host sees. Allow it at
startup:

```bash
"$HOME/.local/share/claude-clipboard/claude-clipboard-host" serve --sync-text --allow-uid 1001
```

Use the UID from the error instead of the example 1001. If SELinux refuses the
connection before the UID check, look at the AVC record: a narrow rule for the
socket may be needed. The `z` file label alone does not guarantee permission to
connect to a host process. There is no need to disable SELinux entirely.

Allowed containers can read the current image while the bridge is running. The
server offers no clipboard write commands and does not export plain text. To
revoke access, stop the server or remove the mount.

## Tests and limitations

The Rust tests run without Wayland or X11; stand-in `wl-paste` and `xsel`
scripts take their place. They cover:

- the protocol: request parsing, image type selection, the magic-byte check,
  framing, and an image at the 64 MiB limit delivered byte-exact through the
  socket;
- the server as it runs: the command line and its exit status, the lock that
  refuses a second instance, replacing a stale socket and refusing any other
  file at its path, the concurrency limit, and a clean exit on SIGTERM, SIGINT
  or SIGHUP that removes the socket at once, turns away a client still sending
  its request and waits for requests already received, holding its lock until
  the last of them is answered, while any of these signals the bridge was
  started with ignored stays ignored;
- the server's checks, as functions rather than through the running server:
  the peer UID check, and a deadline on each connection however slowly the
  peer sends or reads, proved with short deadlines in place of the 12 s ones;
- tools: timeouts, reaping, and leaving alone the processes a tool that
  succeeded started, such as the daemon `xsel` keeps to own the selection;
- text sync: the whole decision sequence, including skipping offers with an
  image and not rewriting text X11 already holds, and the watcher ending when
  the bridge is killed.

The Docker build runs them before building. From the repository root:

```bash
cargo test
```

The Python tests cover the container wrapper's argument translation, and its
reading of the host's response, including a refusal the host sent before the
wrapper's request arrived:

```bash
python3 -m unittest discover -s . -p test_bridge.py -v
```

The host needs Linux 5.3 or later. Pasting end to end into Claude Code inside a
devcontainer has not been verified yet. The check in step 5 separates the bridge working from the behaviour of a
particular CLI or terminal.

## Sources

- [Claude Code: working with images](https://code.claude.com/docs/en/common-workflows#work-with-images)
- [Issue: image paste silently fails when wl-paste is present but non-functional](https://github.com/anthropics/claude-code/issues/85284)
- [Issue: add image/bmp to the Linux clipboard check](https://github.com/anthropics/claude-code/issues/25935)
- [Docker: bind mounts and SELinux](https://docs.docker.com/engine/storage/bind-mounts/)

## License

Copyright 2026 Vladimir Rindevich. Licensed under the Apache License 2.0: see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
