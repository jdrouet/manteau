# fetch the vendor with the builder platform to avoid qemu issues
FROM --platform=$BUILDPLATFORM rust:1-alpine AS vendor

ENV USER=root

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
RUN cargo init
COPY Cargo.lock Cargo.toml /code/
COPY --from=vendor /code/.cargo /code/.cargo
COPY --from=vendor /code/vendor /code/vendor

COPY src /code/src
RUN cargo build --release --offline

FROM alpine

ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info

COPY --from=builder /code/target/release/manteau /manteau

EXPOSE 3000

ENTRYPOINT [ "/manteau" ]
