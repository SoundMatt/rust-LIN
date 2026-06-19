# Copyright (c) 2026 Matt Jones. All rights reserved.
# MPL-2.0
#
# Multi-stage build for the rust-lin binary.
# Stage 1: build on rust:1.80-alpine.
# Stage 2: minimal alpine runtime image.

# ── builder ──────────────────────────────────────────────────────────────────
FROM rust:1.80-alpine AS builder

# musl build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Cache dependency layer separately
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo 'fn main() {}' > src/bin/main.rs && \
    echo 'pub fn lib() {}' > src/lib.rs && \
    cargo build --release --bin rust-lin && \
    rm -f target/release/deps/rust_lin* target/release/rust-lin

# Copy full source and rebuild
COPY . .
RUN cargo build --release --bin rust-lin --locked

# ── runtime ───────────────────────────────────────────────────────────────────
FROM alpine:3.20

LABEL org.opencontainers.image.title="rust-lin" \
      org.opencontainers.image.description="LIN bus RELAY adapter (ASIL-B)" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.licenses="MPL-2.0" \
      org.opencontainers.image.authors="Matt Jones <matt@jellybaby.com>"

# ca-certificates for TLS; no shell in production image is intentional
RUN apk add --no-cache ca-certificates

COPY --from=builder /build/target/release/rust-lin /usr/local/bin/rust-lin

# Non-root user
RUN addgroup -S linbus && adduser -S linbus -G linbus
USER linbus

ENTRYPOINT ["/usr/local/bin/rust-lin"]
CMD ["version", "--format", "json"]
