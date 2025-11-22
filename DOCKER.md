# Docker Setup

## Quick Start

This repository contains everything needed to run a complete home automation stack on your Raspberry Pi.

### 1. Build on your Mac

```bash
# Build the ARM64 binary (much faster than building on Pi)
cargo build --release --target aarch64-unknown-linux-musl
```

### 2. Deploy to Raspberry Pi

```bash
# Option A: Using git (recommended)
# On your Pi:
git clone <your-repo> ~/home-assistant-rs
cd ~/home-assistant-rs

# Option B: Using scp
# On your Mac:
scp -r . ludovic@raspberrypi:~/home-assistant-rs/
ssh ludovic@raspberrypi
cd ~/home-assistant-rs
```

### 3. Start Everything

```bash
# Start all services with one command
docker compose up -d
```

This launches:
- **mosquitto** - MQTT broker
- **zigbee2mqtt** - Zigbee coordinator
- **home-assistant-rs** - This Rust application

## What's Running

All services are accessible on your Raspberry Pi:

- **Home Assistant RS Dashboard**: `http://raspberrypi.local:8082`
- **Zigbee2MQTT Web UI**: `http://raspberrypi.local:8080`
- **MQTT Broker (mosquitto)**: `raspberrypi.local:1883`

## Service Management

### View all logs
```bash
docker compose logs -f
```

### View specific service logs
```bash
docker compose logs -f home-assistant-rs
docker compose logs -f zigbee2mqtt
docker compose logs -f mosquitto
```

### Restart a service
```bash
docker compose restart home-assistant-rs
```

### Stop everything
```bash
docker compose down
```

### Rebuild after code changes
```bash
# Build new binary on Mac
cargo build --release --target aarch64-unknown-linux-musl

# Deploy and rebuild on Pi
docker compose up -d --build
```

## Architecture

All services run in the same Docker network (`home-assistant-network`) and communicate using container names as DNS:

```
┌──────────────────────────────────────────────────┐
│          home-assistant-network (bridge)         │
│                                                  │
│  ┌──────────┐    ┌──────────────┐   ┌─────────┐│
│  │mosquitto │◄───┤ zigbee2mqtt  │   │  home-  ││
│  │  :1883   │    │   :8080      │   │assistant││
│  └────▲─────┘    └──────────────┘   │   -rs   ││
│       │                               │  :8082  ││
│       └───────────────────────────────┴─────────┘│
└──────────────────────────────────────────────────┘
         │            │                    │
         ▼            ▼                    ▼
      1883:1883    8080:8080          8082:8080
     (MQTT)     (Zigbee Web UI)      (Dashboard)
```

## Configuration

### Zigbee USB Adapter

The default configuration expects the Zigbee adapter at `/dev/ttyUSB0`. If yours is different:

1. Find your adapter:
   ```bash
   ls /dev/tty*
   ```

2. Edit `docker-compose.yml`:
   ```yaml
   zigbee2mqtt:
     devices:
       - /dev/ttyACM0:/dev/ttyACM0  # Change here
   ```

### Mosquitto MQTT Broker

Configuration is in `services/mosquitto/config/mosquitto.conf`. The default allows anonymous connections on port 1883.

### Zigbee2MQTT

Configuration is in `services/zigbee2mqtt/data/configuration.yaml`. On first run, zigbee2mqtt will create this file with defaults. The current configuration should already have:
- MQTT server pointing to `mqtt://mosquitto:1883` (using Docker network)
- Serial port set to `/dev/ttyUSB0`
- Frontend enabled on port 8080

You may need to adjust the serial port if your Zigbee adapter uses a different device path.

## Directory Structure

```
home-assistant-rs/
├── docker-compose.yml          # Orchestrates all services
├── services/
│   ├── mosquitto/
│   │   ├── config/             # MQTT broker config (tracked in git)
│   │   ├── data/               # Runtime data (gitignored)
│   │   └── log/                # Logs (gitignored)
│   ├── zigbee2mqtt/
│   │   └── data/               # Device DB & config (gitignored)
│   └── home-assistant-rs/
│       └── data/               # SQLite database (gitignored)
├── src/                        # Rust source code
└── Dockerfile                  # Uses pre-built ARM64 binary
```

## Troubleshooting

### Service won't start
```bash
# Check logs for the specific service
docker compose logs mosquitto
docker compose logs zigbee2mqtt
docker compose logs home-assistant-rs
```

### MQTT connection issues
```bash
# Test MQTT broker is running
docker compose ps

# Check mosquitto logs
docker compose logs mosquitto

# Test connection from host
mosquitto_sub -h localhost -p 1883 -t '#' -v
```

### Zigbee adapter not found
```bash
# Verify device exists
ls -l /dev/ttyUSB0

# Check permissions (may need to add user to dialout group)
sudo usermod -a -G dialout $USER
```

### Reset everything
```bash
# Stop and remove all containers
docker compose down

# Optional: Remove all data (WARNING: deletes databases!)
rm -rf services/*/data/* services/mosquitto/log/*

# Start fresh
docker compose up -d
```
