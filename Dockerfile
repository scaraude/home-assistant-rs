# Lightweight runtime image for Raspberry Pi Zero 2W (ARM64)
# Build the binary on your Mac first with: cargo build --release --target aarch64-unknown-linux-musl
FROM alpine:3.19

WORKDIR /app

# Install wget for health checks
RUN apk add --no-cache wget

# Copy pre-built binary and static files
COPY target/aarch64-unknown-linux-musl/release/home-assistant-rs /app/
COPY static ./static

# Create data directory
RUN mkdir -p /app/data && \
    addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser && \
    chown -R appuser:appuser /app

USER appuser

EXPOSE 8080

CMD ["/app/home-assistant-rs"]
