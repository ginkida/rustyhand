# syntax=docker/dockerfile:1

# ── Stage 1: Build ──────────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY agents ./agents
RUN cargo build --release --bin rustyhand

# ── Stage 2: Runtime ────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /usr/sbin/nologin -d /data rustyhand

COPY --from=builder /build/target/release/rustyhand /usr/local/bin/
COPY agents /opt/rustyhand/agents

# Default config: bind 0.0.0.0 so the port is reachable outside container.
# Overridden when user mounts their own /data/config.toml.
RUN mkdir -p /data && chown rustyhand:rustyhand /data
RUN printf '%s\n' \
    '# Docker default — override by mounting your own config.toml' \
    'api_listen = "0.0.0.0:4200"' \
    '' \
    '[default_model]' \
    'provider = "minimax"' \
    'model = "MiniMax-M2.7"' \
    'api_key_env = "MINIMAX_API_KEY"' \
    > /data/config.toml && chown rustyhand:rustyhand /data/config.toml

USER rustyhand
EXPOSE 4200
VOLUME /data
ENV RUSTY_HAND_HOME=/data
ENV RUSTY_HAND_AGENTS_DIR=/opt/rustyhand/agents

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://127.0.0.1:4200/api/health || exit 1

ENTRYPOINT ["rustyhand"]
CMD ["start"]
