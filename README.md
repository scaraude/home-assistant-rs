# Home Assistant RS

A lightweight home automation service written in Rust, optimized for Raspberry Pi Zero 2W.

## Features

- 🔌 **MQTT Integration**: Listens to zigbee2mqtt events for sensors and switches
- 💾 **SQLite Storage**: Persists sensor readings and device state with minimal overhead
- 🌐 **Web Dashboard**: Modern Svelte UI for monitoring and control
- 🎛️ **Switch Control**: Toggle Zigbee switches via web interface
- 🤖 **Automation Rules**: Create condition-based automation rules with visual editor
- 📊 **System Monitoring**: Real-time CPU, RAM, and process metrics
- 🚀 **Lightweight**: ~3MB binary, ~10-20MB runtime memory
- ⚡ **Async**: Non-blocking I/O for efficient resource usage
- 📡 **Device Discovery**: Automatic registration of new Zigbee devices
- 🔋 **Device State**: Battery level and link quality monitoring

## Architecture

```
        ┌───────────────┐         ┌─────────────┐         ┌─────────────────┐
        │  zigbee2mqtt  │         │  mosquitto  │         │ home-assistant  │
        │   (systemd)   │◄───────►│   (broker)  │◄───────►│      -rs        │
        │               │  pub/sub│  port 1883  │  pub/sub│   (systemd)     │
        │  Web: 8080    │         └─────────────┘         │   Web: 8082     │
        └───────────────┘                                 │                 │
                ▲                                         │  ┌───────────┐  │
                │ USB                                     │  │  SQLite   │  │
                │ /dev/ttyUSB0                            │  │ Database  │  │
                ▼                                         │  └───────────┘  │
        ┌───────────────┐                                 │                 │
        │ Zigbee Dongle │                                 │  ┌───────────┐  │
        │  (Sonoff 3.0) │                                 │  │   Hyper   │  │
        └───────────────┘                                 │  │HTTP Server│  │
                ▲                                         │  └───────────┘  │
                │ Zigbee 3.0                              └────────▲────────┘
                ▼                                                  │ HTTP
        ┌───────────────┐                                          ▼
        │Zigbee Devices │                                  ┌──────────────┐
        │ • Sensors     │                                  │    Svelte    │
        │ • Switches    │                                  │  Dashboard   │
        │ • Lights      │                                  │ (Browser UI) │
        └───────────────┘                                  └──────────────┘
```

## Prerequisites

- **Rust**: 1.83+ (edition 2024)
- **Cross-compilation** (Mac/Linux to Raspberry Pi):
  - Install cross: `cargo install cross`
  - Or: `rustup target add aarch64-unknown-linux-gnu`
- **Frontend** (optional, for development):
  - Node.js 18+ and npm
- **Deployment**: Raspberry Pi with systemd

## Quick Start

### 1. Clone and Build

```bash
# Clone the repository
git clone <repo-url>
cd home-assistant-rs

# Build for Raspberry Pi
make build

# Build frontend
cd frontend && npm install && npm run build
```

### 2. Deploy to Raspberry Pi

```bash
# One-time full deployment (sets up services, transfers files)
make deploy-full

# Start services
make start
```

### 3. Access Dashboard

Open `http://<raspberry-pi-ip>:8082` in your browser.

## Building

### Native Build (Development)

```bash
# Development build
cargo build

# Release build (optimized for size)
cargo build --release

# Run tests
cargo test
```

### Cross-compilation for Raspberry Pi

```bash
# Using Makefile (recommended)
make build

# Or manually with cross
cross build --release --target aarch64-unknown-linux-gnu

# Binary will be at: target/aarch64-unknown-linux-gnu/release/home-assistant-rs
```

The release build uses aggressive optimization for embedded systems:

- `opt-level = "z"` - Optimize for binary size
- `lto = true` - Link-time optimization
- `strip = true` - Strip debug symbols
- `panic = "abort"` - Reduce panic handling overhead

**Result**: ~3MB binary, ~10-20MB runtime memory

## Deployment

### Using Makefile (Recommended)

```bash
# First-time setup (builds, transfers, installs services)
make deploy-full

# After backend changes (quick rebuild + restart)
make quick-deploy

# After frontend changes
make quick-deploy-frontend

# Service management
make start          # Start all services
make stop           # Stop all services
make restart        # Restart home-assistant-rs
make status         # Check service status
make logs           # Tail all logs
make logs-home-assistant  # Tail only home-assistant-rs logs

# Database backup
make backup
make restore
```

### Configuration

Set deployment variables in `.env.deploy` (or use defaults):

```bash
PI_HOST=Gholam.local       # Raspberry Pi hostname/IP
PI_USER=ludovic            # SSH user
RUST_TARGET=aarch64-unknown-linux-gnu
DEPLOY_DIR=/opt/home-assistant-rs
```

### Services

The Makefile sets up 4 systemd services on the Pi:

1. **mosquitto** - MQTT broker (port 1883)
2. **zigbee2mqtt** - Zigbee coordinator bridge (front on port 8080)
3. **home-assistant-rs** - This application (port 8082)
4. **system-monitor** - System metrics collector (runs `monitor.sh`)

All services start automatically on boot and communicate via MQTT on localhost.

## Database Schema

### Device Model (Capability-Based)

Devices have **capabilities** (Sensor or Commander) with specific **subtypes**:

```sql
CREATE TABLE devices (
    id TEXT PRIMARY KEY,                  -- Zigbee device ID
    name TEXT NOT NULL,                   -- User-friendly name
    mqtt_topic TEXT NOT NULL UNIQUE,      -- MQTT topic
    capability_type TEXT NOT NULL,        -- 'sensor' or 'commander'
    capability_subtype TEXT NOT NULL      -- e.g., 'temp_humidity', 'switch'
);
```

### Sensor Data (Time-Series)

```sql
CREATE TABLE temperature_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    temperature REAL NOT NULL,
    humidity REAL NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (device_id) REFERENCES devices(id)
);
CREATE INDEX idx_sensor_time ON temperature_readings(device_id, timestamp DESC);
```

### Device State (Ephemeral)

Separated from time-series data for performance:

```sql
CREATE TABLE device_state (
    device_id TEXT PRIMARY KEY,
    battery_level INTEGER,      -- Battery percentage (0-100)
    link_quality INTEGER,       -- Link quality (0-255)
    last_seen INTEGER NOT NULL, -- Unix timestamp
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);
```

### Automation Rules

```sql
CREATE TABLE automation_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    conditions TEXT NOT NULL,        -- JSON array of conditions
    actions TEXT NOT NULL,           -- JSON array of actions
    logical_operator TEXT NOT NULL   -- 'and' or 'or'
);

CREATE TABLE automation_execution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER NOT NULL,
    executed_at INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
);
```

## API Endpoints

### Devices

- `GET /api/sensors` - List all sensor devices with latest readings
- `GET /api/devices/switches` - List all switches with state, battery, link quality
- `GET /api/devices/{device_id}/state` - Get device state (battery, link quality, last seen)
- `PATCH /api/devices/{device_id}` - Update device name

### Sensor Data

- `GET /api/readings?hours=24` - Get recent readings (default 24 hours)
- `GET /api/readings?device_id={id}&hours=24` - Get readings for specific sensor
- `GET /api/readings?since={timestamp}` - Delta updates (bandwidth optimization)

### Switch Control

- `POST /api/commands/execute` - Execute switch command
  ```json
  {
    "device_id": "0xa4c138abcdef1234",
    "state": true
  }
  ```

### Automation

- `GET /api/automation/rules` - List all automation rules
- `GET /api/automation/rules/{id}` - Get specific rule with details
- `POST /api/automation/rules` - Create new automation rule
- `PUT /api/automation/rules/{id}` - Update existing rule
- `DELETE /api/automation/rules/{id}` - Delete rule
- `GET /api/automation/rules/{id}/executions?limit=100` - Get execution history

### System Monitoring

- `GET /api/logs/files` - List available log files
- `GET /api/logs/view?file={name}` - View log file content
- `GET /api/logs/view?file={name}&since_line={n}` - Delta log updates
- `GET /api/system/top-consumers?type=cpu` - Top CPU consumers
- `GET /api/system/top-consumers?type=ram` - Top RAM consumers
- `GET /api/system/process-history?process={name}&pid={pid}&hours=24` - Process metrics

## Features in Detail

### Device Discovery

New Zigbee devices are automatically discovered and registered:

1. zigbee2mqtt publishes device announcement
2. home-assistant-rs detects new device
3. Device is stored in database with capability metadata
4. Appears in web UI immediately

### Automation System

Create rules with conditions and actions:

**Example Rule**: "Turn on light when temperature drops below 20°C"

```json
{
  "name": "Cold Room Light",
  "enabled": true,
  "logical_operator": "and",
  "conditions": [
    {
      "sensor_id": "0xa4c138temp",
      "field": "temperature",
      "operator": "less_than",
      "value": 20.0
    }
  ],
  "actions": [
    {
      "device_id": "0xa4c138light",
      "action": "on"
    }
  ]
}
```

**Supported Conditions**:

- Sensor thresholds (temperature, humidity)
- Comparison operators: `<`, `>`, `<=`, `>=`, `==`, `!=`
- Time-based conditions (planned)

**Supported Actions**:

- Switch on/off
- Future: Notifications, delays, multi-step sequences

### Performance Optimizations

**Caching**:

- 60-second HTTP response cache with ETag support
- Delta updates for sensor data (99% bandwidth reduction)
- 304 Not Modified responses for unchanged data

**Database**:

- Indexed time-series queries
- Separation of hot data (device_state) from cold data (sensor readings)
- Memory-mapped I/O
- Serialized writes to prevent memory spikes

**Frontend**:

- Client-side caching with delta updates
- 15-second polling with conditional requests
- Chart.js for efficient data visualization

## Development

### Project Structure

```
src/
├── main.rs              # Entry point, runtime initialization
├── mqtt/                # MQTT client and message handlers
├── db/                  # Database layer with query modules
├── http/                # HTTP server (Hyper) and API routes
├── models/              # Data models (Device, SensorReading, etc.)
├── state/               # In-memory state stores
├── automation/          # Rule evaluation engine
├── cache/               # HTTP response cache
└── logs/                # System log parsing

frontend/                # Svelte dashboard
├── src/lib/
│   ├── SensorCard.svelte
│   ├── SwitchCard.svelte
│   ├── AutomationRulePanel.svelte
│   ├── GraphModal.svelte
│   └── api.ts
└── package.json
```

### Running Tests

```bash
# Run all tests (92 tests)
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_automation_rule_evaluation
```

**Test Coverage**:

- Database operations (CRUD, transactions, indexes)
- MQTT message parsing
- Automation rule evaluation
- API endpoint responses
- Foreign key constraints

### Frontend Development

```bash
cd frontend

# Install dependencies
npm install

# Development server (with hot reload)
npm run dev

# Build for production
npm run build

# Type check
npm run check
```

### Why These Technologies?

**Rust**: Memory safety, zero-cost abstractions, perfect for embedded systems
**Hyper over Axum**: 100KB vs 500KB overhead - every byte counts on Pi Zero
**SQLite**: No external database, embedded-friendly, memory-mapped I/O
**Svelte**: Minimal runtime, compiles to vanilla JS, fast on low-power devices
**systemd**: Standard Linux service manager, automatic restarts, logging

## Troubleshooting

### Check Service Status

```bash
make status
# Or manually:
ssh <pi-user>@<pi-host> 'sudo systemctl status home-assistant-rs'
```

### View Logs

```bash
make logs-home-assistant
# Or manually:
ssh <pi-user>@<pi-host> 'sudo journalctl -u home-assistant-rs -f'
```

### Debug MQTT

```bash
# Subscribe to all zigbee2mqtt topics
mosquitto_sub -h localhost -t 'zigbee2mqtt/#' -v
```

### Database Issues

```bash
# Backup database
make backup

# SSH to Pi and inspect
ssh <pi-user>@<pi-host>
sqlite3 /var/lib/home-assistant-rs/database/home_assistant.db
```

## Future Enhancements

- [ ] Multi-sensor support (motion, door, contact sensors)
- [ ] Time-based automation conditions
- [ ] Push notifications (MQTT, email, webhook)
- [ ] Data retention policies (auto-cleanup old readings)
- [ ] User authentication
- [ ] Scene management (save/restore device states)
- [ ] Energy monitoring
- [ ] Mobile app (PWA)

## License

MIT

## Contributing

Pull requests welcome! Please ensure:

- All tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)
- Frontend type-checks (`cd frontend && npm run check`)
