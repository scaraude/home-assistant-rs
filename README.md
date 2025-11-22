# Home Assistant RS

A lightweight home automation service written in Rust, optimized for Raspberry Pi Zero 2W.

## Features

- 🔌 **MQTT Integration**: Listens to zigbee2mqtt events
- 💾 **SQLite Storage**: Persists temperature readings with minimal overhead
- 🌐 **HTTP Server**: Serves a web dashboard for temperature monitoring
- 🚀 **Lightweight**: ~3MB binary, optimized for low-resource systems
- ⚡ **Async**: Non-blocking I/O for efficient resource usage

## Architecture

```
┌─────────────┐      MQTT       ┌──────────────────┐
│ zigbee2mqtt │ ──────────────> │  home-assistant  │
│ (container) │                 │       -rs        │
└─────────────┘                 │                  │
                                │  ┌────────────┐  │
┌─────────────┐                 │  │   SQLite   │  │
│  mosquitto  │ <──────────────>│  └────────────┘  │
│ (container) │      MQTT       │                  │
└─────────────┘                 │  ┌────────────┐  │
                                │  │HTTP Server │  │
                                │  └────────────┘  │
                                └────────┬─────────┘
                                         │
                                         v
                                   Web Dashboard
                                   (localhost:8080)
```

## Prerequisites

- Rust 1.83+ (edition 2021)
- Running mosquitto MQTT broker
- Running zigbee2mqtt instance
- For cross-compilation (Mac to Raspberry Pi):
  - `brew install FiloSottile/musl-cross/musl-cross`
  - `rustup target add aarch64-unknown-linux-musl`

## Building

### Native Build
```bash
# Development build
cargo build

# Release build (optimized for size)
cargo build --release
```

### Cross-compilation for Raspberry Pi (on Mac)
```bash
# Build ARM64 binary for Raspberry Pi
cargo build --release --target aarch64-unknown-linux-musl

# Binary will be at: target/aarch64-unknown-linux-musl/release/home-assistant-rs
```

The release build uses aggressive optimization:
- `opt-level = "z"` - Optimize for binary size
- `lto = true` - Link-time optimization
- `strip = true` - Strip debug symbols
- `panic = "abort"` - Reduce panic handling overhead

### Git Hooks

Install the pre-push hook to automatically build the ARM64 binary before pushing:

```bash
./install-hooks.sh
```

This ensures the binary in `target/aarch64-unknown-linux-musl/release/` is always up-to-date for deployment.

## Running

### Native
```bash
# Development
cargo run

# Production
./target/release/home-assistant-rs
```

### Docker (Raspberry Pi)

1. **On your Mac**: Build the ARM64 binary
   ```bash
   cargo build --release --target aarch64-unknown-linux-musl
   ```

2. **Deploy to Raspberry Pi**: Copy the project or use git, then:
   ```bash
   docker compose up -d
   ```

The Docker image uses the pre-built binary from your Mac (fast) instead of compiling on the Pi (slow).

The service will:
1. Connect to MQTT broker at `localhost:1883`
2. Subscribe to `zigbee2mqtt/#` topics
3. Store temperature readings in `home_assistant.db`
4. Start HTTP server on `http://0.0.0.0:8080`

## Configuration

Currently hardcoded in `src/main.rs`. Future: move to config file or env vars.

```rust
let mqtt_broker = "localhost";
let mqtt_port = 1883;
let http_addr = "0.0.0.0:8080";
let db_path = "home_assistant.db";
```

## API Endpoints

- `GET /` - Web dashboard (HTML)
- `GET /api/sensors` - List all sensor IDs (JSON)
- `GET /api/readings?hours=24` - Get readings from last N hours (JSON)
- `GET /api/readings?sensor_id=<id>&hours=24` - Get readings for specific sensor (JSON)

## Database Schema

```sql
CREATE TABLE temperature_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id TEXT NOT NULL,
    temperature REAL NOT NULL,
    humidity REAL,
    battery INTEGER,
    timestamp INTEGER NOT NULL  -- Unix timestamp
);

CREATE INDEX idx_sensor_time ON temperature_readings(sensor_id, timestamp DESC);
```

## Memory Usage

Optimized for Raspberry Pi Zero 2W (512MB RAM):
- Binary size: ~3.1MB
- Runtime memory: ~10-20MB (depends on sensor count)
- SQLite uses memory-mapped I/O for efficiency

## Development Notes

### Why Hyper instead of Axum?

- Hyper: ~100KB overhead, minimal abstraction
- Axum: ~500KB+ overhead, full web framework

For serving one static page + simple API, Hyper is more appropriate for embedded systems.

### Thread Safety

- SQLite connection wrapped in `Mutex<Connection>` for thread safety
- MQTT events processed in separate task, sent via `mpsc::channel`
- Database writes are serialized (prevents memory spikes on concurrent writes)

## TODO

- [ ] Implement chart rendering in `static/index.html` (see renderChart function)
- [ ] Move configuration to external file or environment variables
- [ ] Add data retention policy (auto-delete old readings)
- [ ] Add authentication for HTTP endpoints
- [ ] Support more sensor types (motion, door, etc.)
- [ ] Add metrics/health endpoint

## License

MIT
