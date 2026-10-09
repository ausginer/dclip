# DClip

DClip lets a program inside a devcontainer read the image on the host's
Wayland clipboard. Inside the container, small Python wrappers stand in for
`wl-paste` and `xclip`, and answer their image reads: listing the clipboard's
types, and reading an image of a given type. On the host, the `dclip` binary
reads the current image from the Fedora Wayland clipboard on request, over a
Unix socket.

DClip was written so that Claude Code can paste images inside a devcontainer.

| Where                  | What it needs                                                        |
| ---------------------- | -------------------------------------------------------------------- |
| Fedora x86_64          | The `dclip` package, or the built binary; the real `wl-paste`; `xsel` for text sync |
| Working devcontainer   | An existing Python 3, the mounted directories, a PATH setting        |
| Build environment      | Docker, or Rust 1.89+ with the musl target                           |

Python, Cargo and rustc do not need to be installed on Fedora. The Python file
is installed on the host but runs only inside the devcontainer. The binary
is built for `x86_64-unknown-linux-musl` and statically linked, so it does not
depend on Fedora's glibc version.

PNG, JPEG, GIF and WebP are supported when the corresponding MIME type is
offered by the clipboard. There is no conversion from other formats. Images are
passed in memory, and no clipboard history is kept. The wrappers export images
only and are not a full replacement for the real clipboard tools. A single
response is limited to 64 MiB.

There are two ways to install DClip on the host: the `dclip` package, or
`setup-host.sh` from a checkout. They differ in where the files go, and so in
what the container mounts. Each step below says where they differ.

## 1. Build

All build commands run from the repository root, in a container or anywhere
else with the tools; nothing Rust-related is needed on Fedora.

**The packages.** Each run of the repository's CI workflow uploads the `.rpm`
and the `.deb` as an artifact named `dclip-packages`. To build them yourself,
with a Docker CLI that can reach a Docker daemon:

```bash
docker build --target packages --output type=local,dest=dist .
```

This runs the Rust tests, builds the release binary, packages it with the
container wrapper, and installs and removes each package in a Fedora and a
Debian image to check it. Only then does it export
`dist/dclip-<version>-1.x86_64.rpm` and `dist/dclip_<version>-1_amd64.deb`.

**The binary alone,** for `setup-host.sh`:

```bash
docker build --target binary --output type=local,dest=dist .
```

The Rust toolchain and the musl target are installed inside the build image. The
build runs the Rust tests first, then builds the release binary and exports it
as `dist/dclip`. If the tests fail, nothing is exported. The
build container does not need Wayland access.

Alternatively, build in an existing Rust environment, such as this repository's
devcontainer. Only the musl target needs adding:

```bash
rustup target add x86_64-unknown-linux-musl
cargo test --target x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

The result is `target/x86_64-unknown-linux-musl/release/dclip`.
Copy it to `dist/dclip`, or pass its path to `setup-host.sh` in
the next step. Either way it has to be somewhere Fedora can read.

## 2. Install on Fedora

**From the package:**

```bash
sudo dnf install ./dclip-0.1.0-1.x86_64.rpm
```

It installs `/usr/bin/dclip` and the container wrapper in `/usr/share/dclip`,
and pulls in `wl-clipboard`, and `xsel` unless weak dependencies are turned
off. It starts and configures nothing. On Debian or Ubuntu, `sudo apt install
./dclip_0.1.0-1_amd64.deb` does the same.

**From a checkout,** without a package:

```bash
bash setup-host.sh
```

A different path to the binary can be passed as the first argument:

```bash
bash setup-host.sh /path/to/dclip
```

This installs the binary, the container wrapper and the `bin/wl-paste` and
`bin/xclip` links into `~/.local/share/dclip/`. Do not add that `bin`
directory to the host's PATH.

If a text-sync watcher such as `wl-paste --watch ...` is already running, stop
it: `--sync-text` below replaces it. Then run in a Fedora terminal, after a
package install:

```bash
dclip serve --sync-text
```

or after `setup-host.sh`:

```bash
"$HOME/.local/share/dclip/dclip" serve --sync-text
```

Either way, `serve` puts its socket at
`$HOME/.local/share/dclip/clipboard.sock`, and creates that directory if it
does not exist. `DCLIP_SOCKET` overrides the path.

Leave the terminal open for now; closing it, Ctrl+C or SIGTERM stops the bridge.
It removes its socket at once and stops its text watcher, answers the requests
it has already received, and tells a client still sending one that the bridge is
shutting down. Any of SIGHUP, SIGINT and SIGTERM that the bridge was started
with ignored stays ignored, so under `nohup`, which ignores SIGHUP, it keeps
running when its terminal closes. `--sync-text` copies plain text into X11 with
`xsel`. It checks the current content first and skips offers that contain an
image, including mixed offers where an image comes with a text representation.
If you do not need this, run `serve` without `--sync-text`.

## 3. Mount the host's directories

Mount whole directories, not just the socket file: that way the server can be
restarted without recreating the containers. A read-only mount is enough: the
container reads the wrapper and connects to the socket, and creates no files on
the host.

**After a package install,** the wrapper and the socket are in two places, so
the container mounts two directories and is told where the socket is:

| Fedora                     | Container    | Mount                         |
| -------------------------- | ------------ | ----------------------------- |
| `/usr/share/dclip`         | `/opt/dclip` | read-only, never `z` or `Z`   |
| `$HOME/.local/share/dclip` | `/run/dclip` | read-only, with `z`           |

and the container's environment sets
`DCLIP_SOCKET=/run/dclip/clipboard.sock`.

Docker Compose, inside the relevant service:

```yaml
volumes:
  - /usr/share/dclip:/opt/dclip:ro
  - ${HOME}/.local/share/dclip:/run/dclip:ro,z
environment:
  DCLIP_SOCKET: /run/dclip/clipboard.sock
```

For a devcontainer using `image` or `build`, extend the existing `runArgs` and
`containerEnv`, and keep the other entries:

```jsonc
"runArgs": [
  "--volume", "/usr/share/dclip:/opt/dclip:ro",
  "--volume", "${localEnv:HOME}/.local/share/dclip:/run/dclip:ro,z"
],
"containerEnv": {
  "DCLIP_SOCKET": "/run/dclip/clipboard.sock"
}
```

`/usr/share/dclip` belongs to the package, so it is mounted without
relabelling: `z` would relabel files the package database tracks. Whether a
container under enforcing SELinux may run the wrapper from there has not been
verified yet.

**After `setup-host.sh`,** the wrapper and the socket share one directory, and
the wrapper finds the socket beside itself:

| Fedora                     | Container    | Mount               |
| -------------------------- | ------------ | ------------------- |
| `$HOME/.local/share/dclip` | `/opt/dclip` | read-only, with `z` |

Docker Compose:

```yaml
volumes:
  - ${HOME}/.local/share/dclip:/opt/dclip:ro,z
```

A devcontainer:

```jsonc
"runArgs": [
  "--volume",
  "${localEnv:HOME}/.local/share/dclip:/opt/dclip:ro,z"
]
```

`z` applies a shared SELinux label, usable by several containers. Do not use `Z`
for this shared directory. Docker `--mount` does not support relabelling, which
is why the examples use `--volume`. Recreate the container after changing its
mounts.

## 4. PATH in the container

In a new container terminal:

```bash
export PATH="/opt/dclip/bin:$PATH"
command -v wl-paste xclip
```

Expect `/opt/dclip/bin/wl-paste` and `/opt/dclip/bin/xclip`.
To make it permanent, add the export to the container user's shell
configuration, or set PATH through the image, Compose or devcontainer settings.
There is no need to set a dummy `DISPLAY` or `WAYLAND_DISPLAY`.

## 5. Check pasting

On Fedora, copy a screenshot as an image. In the container:

```bash
wl-paste --list-types
wl-paste --type image/png | wc -c
```

For a PNG, the first command should list `image/png` and the second should print
a non-zero size. For a JPEG, use `image/jpeg`. If both do, DClip works.

To see which calls a program makes, enable tracing in the terminal it starts
from:

```bash
export DCLIP_TRACE=/tmp/dclip-trace.log
```

The log contains only tool names and arguments, never clipboard content.

**In Claude Code.** Start Claude Code from the same terminal, or resume a
session, and press Ctrl+V. An image attachment should appear. A Claude Code
session that is already running does not see the new PATH, so restart or
resume it. If reading works but Claude Code does not paste the image, enable
tracing before starting it, paste again, and check the file: it shows how a
given Claude Code version calls the tools. If the file does not appear, the
terminal may have intercepted Ctrl+V, or Claude Code reads the clipboard some
other way.

## 6. Autostart in niri

Once the check passes, add one of these at the top level of `config.kdl`. After
a package install:

```kdl
spawn-at-startup "/usr/bin/dclip" "serve" "--sync-text"
```

After `setup-host.sh`:

```kdl
spawn-at-startup "sh" "-c" "exec \"$HOME/.local/share/dclip/dclip\" serve --sync-text"
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
dclip serve --sync-text --allow-uid 1001
```

or, after `setup-host.sh`, the same options to `"$HOME/.local/share/dclip/dclip"`.

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
- the server as it runs: the command line and its exit status, the default
  socket and the directory it creates for it, the lock that refuses a second
  instance, replacing a stale socket and refusing any other
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

From the repository root:

```bash
cargo test
```

The Python tests cover the container wrapper's argument translation, and its
reading of the host's response, including a refusal the host sent before the
wrapper's request arrived:

```bash
python3 -m unittest discover -s . -p test_bridge.py -v
```

**CI.** Every push and pull request runs the repository's GitHub Actions
workflow. Each of its jobs builds one target of the root `Dockerfile`, so any of
them can be rerun locally with nothing but Docker:

| Target      | What it checks                                                       |
| ----------- | -------------------------------------------------------------------- |
| `fmt`       | `cargo fmt --check`                                                  |
| `clippy`    | `cargo clippy --workspace --all-targets -- -D warnings`              |
| `test`      | `cargo test --workspace`                                             |
| `musl`      | the same tests for `x86_64-unknown-linux-musl`, then the release binary |
| `python`    | the container wrapper's tests                                        |
| `guard`     | the tests of the repository's agent tooling                          |
| `check-rpm` | the `.rpm` installs on Fedora with its dependencies and nothing else, carries exactly its files, serves through its own wrapper, and removes cleanly |
| `check-deb` | the same for the `.deb` on Debian, without the end-to-end run        |

For example, `docker build --target check-rpm .`. Once all of them pass, the
workflow uploads the packages the checks installed.

The host needs Linux 5.3 or later. Pasting end to end into Claude Code inside a
devcontainer has not been verified yet. The check in step 5 separates DClip
working from the behaviour of a particular program or terminal.

## Sources

Claude Code's documentation and issues, on how it reads the clipboard:

- [Claude Code: working with images](https://code.claude.com/docs/en/common-workflows#work-with-images)
- [Issue: image paste silently fails when wl-paste is present but non-functional](https://github.com/anthropics/claude-code/issues/85284)
- [Issue: add image/bmp to the Linux clipboard check](https://github.com/anthropics/claude-code/issues/25935)

On the container's mounts:

- [Docker: bind mounts and SELinux](https://docs.docker.com/engine/storage/bind-mounts/)

## License

DClip is copyright 2026 Vladimir Rindevich. It is licensed under the Apache License 2.0: see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
