# Multi-stage Dockerfile for Raspberry Pi Zero 2W (ARM64)
FROM rust:1.83-bookworm as builder

# Install ARM64 cross-compilation tools
RUN apt-get update && \
    apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu && \
    rm -rf /var/lib/apt/lists/*

# Add ARM64 target
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /app

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Configure linker for cross-compilation
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc

# Build for ARM64
RUN cargo build --release --target aarch64-unknown-linux-musl

# Runtime image - minimal Alpine for ARM64
FROM alpine:3.19

WORKDIR /app

# Copy binary and static files
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/home-assistant-rs /app/
COPY static ./static

# Create data directory
RUN mkdir -p /app/data && \
    addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser && \
    chown -R appuser:appuser /app

USER appuser

EXPOSE 8080

CMD ["/app/home-assistant-rs"]
