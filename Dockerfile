# ── Stage 1: Build ──────────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

WORKDIR /build

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/noti-cli/Cargo.toml crates/noti-cli/Cargo.toml
COPY crates/noti-core/Cargo.toml crates/noti-core/Cargo.toml
COPY crates/noti-providers/Cargo.toml crates/noti-providers/Cargo.toml
COPY crates/noti-queue/Cargo.toml crates/noti-queue/Cargo.toml
COPY crates/noti-server/Cargo.toml crates/noti-server/Cargo.toml

# Create dummy source files so cargo can resolve the workspace
RUN mkdir -p crates/noti-cli/src && echo "fn main(){}" > crates/noti-cli/src/main.rs \
 && mkdir -p crates/noti-core/src && echo "" > crates/noti-core/src/lib.rs \
 && mkdir -p crates/noti-providers/src && echo "" > crates/noti-providers/src/lib.rs \
 && mkdir -p crates/noti-queue/src && echo "" > crates/noti-queue/src/lib.rs \
 && mkdir -p crates/noti-server/src && echo "fn main(){}" > crates/noti-server/src/main.rs

# Build dependencies only (cached unless Cargo.toml/Cargo.lock change)
RUN cargo build --release --workspace 2>/dev/null || true

# Copy real source and rebuild
COPY crates/ crates/
RUN touch crates/*/src/main.rs crates/*/src/lib.rs 2>/dev/null || true
RUN cargo build --release --bin noti-server --bin noti

# ── Stage 2: Runtime ────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 noti && useradd --uid 1000 --gid noti --create-home noti

COPY --from=builder /build/target/release/noti-server /usr/local/bin/noti-server
COPY --from=builder /build/target/release/noti /usr/local/bin/noti

# Default data directory for SQLite queue
RUN mkdir -p /data && chown noti:noti /data
VOLUME ["/data"]

USER noti
WORKDIR /data

# Default environment
ENV NOTI_HOST=0.0.0.0 \
    NOTI_PORT=3000 \
    NOTI_LOG_FORMAT=json \
    NOTI_QUEUE_BACKEND=sqlite \
    NOTI_QUEUE_DB_PATH=/data/noti-queue.db

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/noti-server", "--version"]

ENTRYPOINT ["noti-server"]
