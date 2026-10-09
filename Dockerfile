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

