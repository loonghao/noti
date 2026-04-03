# ── Stage 1: Build ──────────────────────────────────────────────
# Multi-arch support: build with `docker buildx build --platform linux/amd64,linux/arm64`
FROM --platform=$BUILDPLATFORM rust:1.85-bookworm AS builder

ARG TARGETPLATFORM
ARG BUILDPLATFORM

# Install cross-compilation toolchain for arm64 when building on amd64
RUN case "$TARGETPLATFORM" in \
      "linux/arm64") \
        apt-get update && apt-get install -y --no-install-recommends \
          gcc-aarch64-linux-gnu libc6-dev-arm64-cross \
        && rm -rf /var/lib/apt/lists/* \
        && rustup target add aarch64-unknown-linux-gnu \
        ;; \
    esac

# Set the correct Cargo target and linker for cross-compilation
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

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
RUN case "$TARGETPLATFORM" in \
      "linux/arm64") CARGO_TARGET="--target aarch64-unknown-linux-gnu" ;; \
      *) CARGO_TARGET="" ;; \
    esac && cargo build --release --workspace $CARGO_TARGET 2>/dev/null || true

# Copy real source and rebuild
COPY crates/ crates/
RUN touch crates/*/src/main.rs crates/*/src/lib.rs 2>/dev/null || true
RUN case "$TARGETPLATFORM" in \
      "linux/arm64") CARGO_TARGET="--target aarch64-unknown-linux-gnu" ;; \
      *) CARGO_TARGET="" ;; \
    esac && cargo build --release --bin noti-server --bin noti $CARGO_TARGET

# Copy binaries to a known location regardless of target triple
RUN mkdir -p /build/out && \
    case "$TARGETPLATFORM" in \
      "linux/arm64") \
        cp /build/target/aarch64-unknown-linux-gnu/release/noti-server /build/out/ && \
        cp /build/target/aarch64-unknown-linux-gnu/release/noti /build/out/ ;; \
      *) \
        cp /build/target/release/noti-server /build/out/ && \
        cp /build/target/release/noti /build/out/ ;; \
    esac

# ── Stage 2: Runtime ────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 noti && useradd --uid 1000 --gid noti --create-home noti

COPY --from=builder /build/out/noti-server /usr/local/bin/noti-server
COPY --from=builder /build/out/noti /usr/local/bin/noti

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
    CMD ["curl", "-sf", "http://localhost:3000/health"]

ENTRYPOINT ["noti-server"]
