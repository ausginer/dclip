# Every gate and every product of DClip, one target each. Run from the
# repository root with nothing but Docker, for example:
#
#   docker build --target binary --output type=local,dest=dist .
#
# Each image is pinned by tag and digest, so that a gate fails only on a change
# to this repository.

# The workspace, built and tested as an unprivileged user, as a developer
# builds it.
FROM rust:1.99.0-slim-trixie@sha256:24e632c09342c20abf8312cf4f61430a911c01ed3a5e4c02b87292b1c39c5273 AS rust
RUN rustup component add rustfmt clippy \
    && rustup target add x86_64-unknown-linux-musl \
    && useradd --create-home builder
USER builder
WORKDIR /home/builder/src
COPY --chown=builder Cargo.toml Cargo.lock .rustfmt.toml clippy.toml ./
COPY --chown=builder crates/host crates/host

FROM rust AS fmt
RUN cargo fmt --check

FROM rust AS clippy
RUN cargo clippy --locked --workspace --all-targets -- -D warnings

FROM rust AS test
RUN cargo test --locked --workspace

# The musl tests, then the release binary from the same stage, so that the
# binary shipped is the one whose tests passed.
FROM rust AS musl
RUN cargo test --locked --workspace --target x86_64-unknown-linux-musl \
    && cargo build --locked --release --target x86_64-unknown-linux-musl

FROM scratch AS binary
COPY --from=musl /home/builder/src/target/x86_64-unknown-linux-musl/release/dclip /dclip

# The container shim's suite, as an unprivileged user.
FROM python:3.14.8-slim-trixie@sha256:a2b82f3c48559aa0a8446d9af49826b6e2b2016f4cd2afabfe6013ec53729170 AS python
RUN useradd --create-home tester
USER tester
WORKDIR /home/tester/src
COPY --chown=tester bridge.py test_bridge.py ./
RUN python3 -m unittest discover -s . -p test_bridge.py -v

# The effort guard's suite. Node runs its TypeScript directly. The suite reads
# the role definitions, the project settings, the marketplace and the launcher.
FROM node:26.11.1-trixie-slim@sha256:193fe51b64e77981119c98c2002c9e32a70e2f006fb4d25068ce0558998917f0 AS guard
USER node
WORKDIR /home/node/src
COPY --chown=node .claude/agents .claude/agents
COPY --chown=node .claude/plugins .claude/plugins
COPY --chown=node .claude/settings.json .claude/settings.json
COPY --chown=node .claude-plugin .claude-plugin
COPY --chown=node .scripts .scripts
RUN node --test .claude/plugins/harness-effort-guard/tests/
