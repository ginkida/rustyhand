# syntax=docker/dockerfile:1
#
# Local-development Dockerfile. Compiles RustyHand from source inside
# the build stage, so a single `docker build .` works on any host
# without needing prebuilt binaries staged in `./bin/`. Slow on multi-
# arch (cargo build inside QEMU is 5-10x slower than native), so the
# CI release flow uses `Dockerfile.release` instead, which COPYs in
# the binaries already produced by the `cli` matrix in
# `.github/workflows/release.yml`. See that file for context.

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

# Pin uid/gid to 1000 so host volumes stay readable across image
# rebuilds. A bare `useradd -r` assigns a dynamic system uid that
# drifts between builds, which breaks a mounted /data from a prior
# image version when the new user cannot write files owned by the
# old one.
RUN groupadd -r -g 1000 rustyhand \
    && useradd -r -u 1000 -g 1000 -s /usr/sbin/nologin -d /data rustyhand

COPY --from=builder /build/target/release/rustyhand /usr/local/bin/
COPY agents /opt/rustyhand/agents
COPY docker-entrypoint.sh /usr/local/bin/

RUN mkdir -p /data && chown rustyhand:rustyhand /data

USER rustyhand
EXPOSE 4200
VOLUME /data
ENV RUSTY_HAND_HOME=/data
ENV RUSTY_HAND_AGENTS_DIR=/opt/rustyhand/agents
ENV RUSTYHAND_CONTAINER=1

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://127.0.0.1:4200/api/health || exit 1

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["start"]
