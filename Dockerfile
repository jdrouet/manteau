# fetch the vendor with the builder platform to avoid qemu issues
FROM --platform=$BUILDPLATFORM rust:1-alpine AS vendor

ENV USER=root

WORKDIR /code/indexer-manager
RUN cargo init
COPY indexer-manager/Cargo.toml /code/indexer-manager/Cargo.toml

WORKDIR /code
RUN cargo init
COPY Cargo.toml /code/Cargo.toml
COPY Cargo.lock /code/Cargo.lock

# https://docs.docker.com/engine/reference/builder/#run---mounttypecache
RUN --mount=type=cache,target=$CARGO_HOME/git,sharing=locked \
    --mount=type=cache,target=$CARGO_HOME/registry,sharing=locked \
    mkdir -p /code/.cargo \
    && cargo vendor > /code/.cargo/config

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

ENV USER=root

WORKDIR /code/indexer-manager
RUN cargo init
COPY indexer-manager/Cargo.toml /code/indexer-manager/Cargo.toml

WORKDIR /code
RUN cargo init
COPY Cargo.toml /code/Cargo.toml
COPY Cargo.lock /code/Cargo.lock

COPY --from=vendor /code/.cargo /code/.cargo
COPY --from=vendor /code/vendor /code/vendor

COPY indexer-manager/src /code/indexer-manager/src
COPY src /code/src
RUN --mount=type=cache,target=/code/target/release/.fingerprint,sharing=private \
    --mount=type=cache,target=/code/target/release/build,sharing=private \
    --mount=type=cache,target=/code/target/release/deps,sharing=private \
    --mount=type=cache,target=/code/target/release/examples,sharing=private \
    --mount=type=cache,target=/code/target/release/incremental,sharing=private \
    cargo build --release --offline

FROM alpine

ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info

COPY --from=builder /code/target/release/manteau /manteau

EXPOSE 3000

ENTRYPOINT [ "/manteau" ]
