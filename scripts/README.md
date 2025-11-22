# Testing Scripts

This directory contains scripts to help you test your home-assistant-rs system on the Raspberry Pi.

## Quick Start

### 1. Send a single test message
```bash
./scripts/quick-test.sh
# Or with custom values:
./scripts/quick-test.sh bedroom_sensor 21.5
```

### 2. Run comprehensive tests
```bash
./scripts/test_mqtt.sh
```

### 3. Simulate continuous sensor data
```bash
# Install Python dependency first:
pip3 install paho-mqtt

# Run simulator:
./scripts/simulate_sensors.py

# With custom settings:
./scripts/simulate_sensors.py --broker localhost --interval 10 --sensors 5
```

### 4. Monitor your system
```bash
./scripts/monitor.sh
```

## Scripts Overview

| Script | Purpose | Use Case |
|--------|---------|----------|
| `quick-test.sh` | Send a single MQTT message | Quick verification |
| `test_mqtt.sh` | Run comprehensive test suite | Test various scenarios |
| `simulate_sensors.py` | Continuous sensor simulation | Load testing, development |
| `monitor.sh` | Monitor MQTT and app logs | Debugging, observation |

## Installation on Raspberry Pi

```bash
# Install MQTT clients
sudo apt-get update
sudo apt-get install mosquitto-clients

# Install Python MQTT library
pip3 install paho-mqtt

# Make scripts executable (if needed)
chmod +x scripts/*.sh scripts/*.py
```

## Configuration

All scripts respect these environment variables:

```bash
export MQTT_BROKER=localhost  # or your broker IP
export MQTT_PORT=1883
```

## Example Testing Workflow

### Complete E2E Test

```bash
# Terminal 1: Start monitoring
./scripts/monitor.sh
# Choose option 5 (Both MQTT + App logs)

# Terminal 2: Run tests
./scripts/test_mqtt.sh

# Terminal 3: Check API
curl http://localhost:8082/api/temperature/latest | jq
```

### Load Testing

```bash
# Start continuous simulation
./scripts/simulate_sensors.py --interval 1 --sensors 10

# In another terminal, monitor performance
docker stats home-assistant-rs

# Check database size
ls -lh data/
```

### Debug Specific Sensor

```bash
# Terminal 1: Monitor specific sensor
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/bedroom_sensor" -v

# Terminal 2: Send test data
./scripts/quick-test.sh bedroom_sensor 20.0

# Terminal 3: Check if it was stored
curl http://localhost:8082/api/temperature/sensor/bedroom_sensor | jq
```

## Troubleshooting

### Scripts can't connect to MQTT
```bash
# Check Mosquitto is running
systemctl status mosquitto
# or for Docker:
docker ps | grep mosquitto

# Test connection manually
mosquitto_pub -h localhost -p 1883 -t "test" -m "hello"
```

### Python script fails
```bash
# Check paho-mqtt is installed
pip3 list | grep paho-mqtt

# Install if missing
pip3 install paho-mqtt
```

### No data in API
```bash
# Check app logs
docker logs home-assistant-rs

# Verify message format
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/#" -v

# Ensure messages contain "temperature" field
```

## Advanced Usage

### Custom Sensor Profiles

Edit `simulate_sensors.py` to add custom sensor behavior:

```python
# Around line 101, modify the sensor names and base values:
sensor_names = [
    "garage_freezer",      # Low temp sensor
    "attic_sensor",        # High temp sensor
    "basement_humid",      # High humidity
]

# Customize base values:
base_temp = random.uniform(5.0, 8.0)  # Freezer range
```

### Stress Testing

```bash
# High-frequency updates
./scripts/simulate_sensors.py --interval 0.5 --sensors 20

# Monitor system resources
htop
# or
docker stats
```

### Integration with CI/CD

```bash
# Run tests and check exit code
./scripts/test_mqtt.sh
sleep 5  # Wait for processing

# Verify data via API
RESPONSE=$(curl -s http://localhost:8082/api/temperature/latest)
if echo "$RESPONSE" | jq -e '.[] | select(.sensor_id == "test_sensor_1")' > /dev/null; then
    echo "✓ Test passed"
    exit 0
else
    echo "✗ Test failed"
    exit 1
fi
```

## See Also

- [TESTING.md](../TESTING.md) - Comprehensive testing guide
- [README.md](../README.md) - Main project documentation
- [DOCKER.md](../DOCKER.md) - Docker deployment guide
