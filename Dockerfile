# fetch the vendor with the builder platform to avoid qemu issues
FROM --platform=$BUILDPLATFORM rust:1-alpine AS vendor

ENV USER=root

WORKDIR /code/indexer-1337x
RUN cargo init
COPY indexer-1337x/Cargo.toml /code/indexer-1337x/Cargo.toml

WORKDIR /code/indexer-bitsearch
RUN cargo init
COPY indexer-bitsearch/Cargo.toml /code/indexer-bitsearch/Cargo.toml

WORKDIR /code/indexer-helper
RUN cargo init
COPY indexer-helper/Cargo.toml /code/indexer-helper/Cargo.toml

WORKDIR /code/indexer-thepiratebay
RUN cargo init
COPY indexer-thepiratebay/Cargo.toml /code/indexer-thepiratebay/Cargo.toml

WORKDIR /code/indexer-manager
RUN cargo init
COPY indexer-manager/Cargo.toml /code/indexer-manager/Cargo.toml

WORKDIR /code/indexer-prelude
RUN cargo init
COPY indexer-prelude/Cargo.toml /code/indexer-prelude/Cargo.toml

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

WORKDIR /code

COPY indexer-1337x/src /code/indexer-1337x/src
COPY indexer-1337x/Cargo.toml /code/indexer-1337x/Cargo.toml

COPY indexer-bitsearch/src /code/indexer-bitsearch/src
COPY indexer-bitsearch/Cargo.toml /code/indexer-bitsearch/Cargo.toml

COPY indexer-helper/src /code/indexer-helper/src
COPY indexer-helper/Cargo.toml /code/indexer-helper/Cargo.toml

COPY indexer-thepiratebay/src /code/indexer-thepiratebay/src
COPY indexer-thepiratebay/Cargo.toml /code/indexer-thepiratebay/Cargo.toml

COPY indexer-manager/src /code/indexer-manager/src
COPY indexer-manager/Cargo.toml /code/indexer-manager/Cargo.toml

COPY indexer-prelude/src /code/indexer-prelude/src
COPY indexer-prelude/Cargo.toml /code/indexer-prelude/Cargo.toml

COPY src /code/src
COPY Cargo.lock Cargo.toml /code/

COPY --from=vendor /code/.cargo /code/.cargo
COPY --from=vendor /code/vendor /code/vendor

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

ENV CONFIG_FILE=/var/lib/manteau/config.toml

COPY config.toml /var/lib/manteau/config.toml
COPY --from=builder /code/target/release/manteau /manteau

EXPOSE 3000

ENTRYPOINT [ "/manteau" ]
