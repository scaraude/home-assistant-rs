# Docker Setup

## Quick Start

On your Raspberry Pi (you already have mosquitto and zigbee2mqtt running):

```bash
# Copy the project to your Pi
scp -r . ludovic@raspberrypi:~/home-assistant-rs/

# SSH to the Pi
ssh ludovic@raspberrypi

# Go to the project directory
cd ~/home-assistant-rs

# Start the app
docker compose up -d
```

## What's Running

The app will connect to your existing mosquitto container on port 1883.

- **Dashboard**: `http://raspberrypi.local:8082`
- **Zigbee2MQTT UI**: `http://raspberrypi.local:8080` (already running)

> Note: Using port 8082 for the dashboard since zigbee2mqtt is already on 8080

## View Logs

```bash
docker compose logs -f
```

## Rebuild After Code Changes

```bash
docker compose up -d --build
```

## Stop

```bash
docker compose down
```

## Configuration

The app connects to mosquitto at `172.17.0.1:1883` (Docker bridge network).

If your mosquitto is on a different network, edit `docker-compose.yml`:

```yaml
environment:
  - MQTT_BROKER=<mosquitto-container-ip>
  - MQTT_PORT=1883
```

To find mosquitto's IP:
```bash
docker inspect mosquitto | grep IPAddress
```
