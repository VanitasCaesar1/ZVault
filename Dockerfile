FROM rust:1.87-bookworm AS builder

RUN apt-get update && apt-get install -y \
    libclang-dev clang pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --package vaultrs-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r vaultrs && useradd -r -g vaultrs vaultrs
RUN mkdir -p /data && chown vaultrs:vaultrs /data

COPY --from=builder /app/target/release/vaultrs-server /usr/local/bin/vaultrs-server

USER vaultrs
EXPOSE 8200

ENV VAULTRS_STORAGE=memory \
    VAULTRS_STORAGE_PATH=/data \
    VAULTRS_DISABLE_MLOCK=true \
    VAULTRS_LOG_LEVEL=info

ENTRYPOINT ["vaultrs-server"]
