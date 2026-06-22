# ── Stage 1: Build ─────────────────────────────────────────────────────────
FROM rust:1.88-slim-bookworm AS builder

# Install system deps needed for keyring + TLS
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libdbus-1-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release

# ── Stage 2: Minimal runtime image ─────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libdbus-1-3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/alexa-cli /usr/local/bin/alexa-cli

ENTRYPOINT ["alexa-cli"]
