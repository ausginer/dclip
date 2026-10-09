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

# The .deb and the .rpm, from the musl stage's binary and the tracked files.
# Both carry the workspace version, the one `version` line in Cargo.toml.
FROM goreleaser/nfpm:v2.47.0@sha256:a662cb167d7b6d3a83920c83d76b12d02b8ac5dd2c13e5c62c15270b23f6df0c AS package
WORKDIR /src
COPY Cargo.toml bridge.py LICENSE NOTICE README.md ./
COPY packaging packaging
COPY --from=musl /home/builder/src/target/x86_64-unknown-linux-musl/release/dclip dist/dclip
RUN DCLIP_VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml) \
    && test -n "$DCLIP_VERSION" \
    && export DCLIP_VERSION \
    && mkdir /out \
    && nfpm package --config packaging/nfpm.yaml --packager deb --target /out/ \
    && nfpm package --config packaging/nfpm.yaml --packager rpm --target /out/

FROM scratch AS packages
COPY --from=package /out/ /

# The install-and-remove check of the .rpm, with the packaged layout run end
# to end, on the current Fedora release.
FROM fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80 AS check-rpm
COPY Cargo.toml bridge.py LICENSE NOTICE README.md /check/repo/
COPY packaging/check-lib.sh packaging/check-rpm.sh /check/
COPY --from=package /out/ /check/packages/
RUN bash /check/check-rpm.sh /check/packages /check/repo

# The install-and-remove check of the .deb, on the current Debian stable.
FROM debian:13.7@sha256:913f6706df59a68922d1dd08f78c2476560a8d367897200a6005b00e5f67c2d5 AS check-deb
COPY Cargo.toml bridge.py LICENSE NOTICE README.md /check/repo/
COPY packaging/check-lib.sh packaging/check-deb.sh /check/
COPY --from=package /out/ /check/packages/
RUN bash /check/check-deb.sh /check/packages /check/repo
