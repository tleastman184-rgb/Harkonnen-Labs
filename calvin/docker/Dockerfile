# ── Stage 1: builder ─────────────────────────────────────────────────────────
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests so dependency layers cache before source changes
COPY Cargo.toml Cargo.lock ./
COPY calvin/Cargo.toml ./calvin/
# Stub all workspace members so cargo can resolve the workspace
RUN mkdir -p src calvin/src && \
    echo 'fn main(){}' > src/main.rs && \
    echo 'fn main(){}' > calvin/src/main.rs

RUN cargo build --release -p calvin-server 2>/dev/null || true

# Copy real source
COPY src/ ./src/
COPY calvin/src/ ./calvin/src/
COPY factory/calvin_archive/ ./factory/calvin_archive/

RUN touch src/main.rs calvin/src/main.rs && \
    cargo build --release -p calvin-server

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/calvin-server /usr/local/bin/calvin-server

EXPOSE 7171

ENTRYPOINT ["/usr/local/bin/calvin-server"]
