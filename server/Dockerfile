# ── Stage 1: Build ───────────────────────────────────────────────────────────
FROM rust:1.76-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies first (layer caching)
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Build the real source
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/codocs /app/codocs
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

CMD ["/app/codocs"]
