# Build from the repository root:
#   docker build -f crates/host/Dockerfile --output type=local,dest=dist .
FROM rust:1-bookworm AS builder
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/host/Cargo.toml crates/host/
COPY crates/host/src crates/host/src
COPY crates/host/tests crates/host/tests
RUN cargo test --locked --target x86_64-unknown-linux-musl \
    && cargo build --locked --release --target x86_64-unknown-linux-musl

FROM scratch AS artifact
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/dclip /dclip
