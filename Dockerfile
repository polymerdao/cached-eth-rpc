FROM rust:alpine3.20 as builder

# Install build dependencies
RUN apk update && apk add --no-cache \
    build-base \
    gcc \
    musl-dev \
    linux-headers \
    libressl-dev \
    pkgconfig \
    && rm -rf /var/cache/apk/*

WORKDIR /app/

COPY src ./src
COPY Cargo.toml .
COPY Cargo.lock .

RUN cargo update -p time # fix a type regression
RUN cargo build --release

FROM alpine:3.20
RUN apk update && apk add --no-cache \
    ca-certificates \
    && rm -rf /var/cache/apk/*
# openssl?

COPY --from=builder /app/target/release/cached-eth-rpc /app/cached-eth-rpc

ENV ENDPOINTS="eth-chain=https://rpc.ankr.com/eth,bsc-chain=https://rpc.ankr.com/bsc"

EXPOSE 8124
ENTRYPOINT [ "/app/cached-eth-rpc" ]
