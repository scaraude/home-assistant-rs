# Testing Examples

Real-world examples of testing your home-assistant-rs system.

## Architecture

```
┌─────────────────┐
│  Zigbee Sensors │  ← Real hardware (when you have it)
└────────┬────────┘
         │ Zigbee
         ↓
┌─────────────────┐
│  Zigbee2MQTT    │  ← Publishes to: zigbee2mqtt/<sensor_id>
└────────┬────────┘
         │ MQTT
         ↓
┌─────────────────┐
│   Mosquitto     │  ← MQTT Broker (port 1883)
└────────┬────────┘
         │ MQTT (subscribe to zigbee2mqtt/#)
         ↓
┌─────────────────┐
│ home-assistant  │  ← Your Rust app (port 8082)
│      -rs        │     Stores in SQLite
└────────┬────────┘
         │ HTTP API
         ↓
┌─────────────────┐
│   Your Browser  │
│   or Scripts    │
└─────────────────┘
```

## For Testing (without real sensors)

```
┌──────────────────────┐
│  Testing Scripts     │  ← Scripts in this directory
│  - quick-test.sh     │     Publish directly to Mosquitto
│  - test_mqtt.sh      │     Bypassing Zigbee2MQTT
│  - simulate_*.py     │
└──────────┬───────────┘
           │ MQTT publish
           ↓
    ┌─────────────────┐
    │   Mosquitto     │
    └────────┬────────┘
             │
             ↓
    ┌─────────────────┐
    │ home-assistant  │
    │      -rs        │
    └─────────────────┘
```

## Example 1: First Time Setup Test

```bash
# Step 1: Start your containers
docker-compose up -d

# Step 2: Verify containers are running
docker ps
# Should see: home-assistant-rs

# Step 3: Check logs
docker logs home-assistant-rs
# Should see: "Starting server on 0.0.0.0:8080"

# Step 4: Send a test message
./scripts/quick-test.sh

# Step 5: Verify it was received
docker logs home-assistant-rs | tail -10
# Look for: "Received temperature reading from test_sensor"

# Step 6: Query the API
curl http://localhost:8082/api/temperature/latest
# Should return JSON with your test reading
```

## Example 2: Simulating Multiple Rooms

```bash
# Simulate a house with 5 sensors
./scripts/simulate_sensors.py --sensors 5 --interval 10
```

**Output:**
```
✓ Connected to MQTT broker
✓ Created sensor: living_room_temp
✓ Created sensor: bedroom_temp
✓ Created sensor: kitchen_temp
✓ Created sensor: bathroom_temp
✓ Created sensor: office_temp

Simulating 5 sensors, publishing every 10s
Press Ctrl+C to stop

[14:23:45] living_room_temp     →  22.3°C   58.2%  Battery:  95%
[14:23:45] bedroom_temp         →  19.8°C   62.5%  Battery:  87%
[14:23:45] kitchen_temp         →  23.5°C   55.1%  Battery: 100%
[14:23:45] bathroom_temp        →  21.1°C   68.7%  Battery:  92%
[14:23:45] office_temp          →  20.2°C   51.3%  Battery:  78%
```

Then query your API:
```bash
curl http://localhost:8082/api/temperature/latest | jq
```

**Response:**
```json
[
  {
    "sensor_id": "living_room_temp",
    "temperature": 22.3,
    "humidity": 58.2,
    "battery": 95,
    "timestamp": 1234567890
  },
  {
    "sensor_id": "bedroom_temp",
    "temperature": 19.8,
    "humidity": 62.5,
    "battery": 87,
    "timestamp": 1234567890
  },
  ...
]
```

## Example 3: Testing Edge Cases

```bash
# Run the comprehensive test suite
./scripts/test_mqtt.sh
```

This tests:
- ✓ Normal readings (temperature + humidity + battery)
- ✓ Minimal readings (temperature only)
- ✓ Multiple sensors simultaneously
- ✓ Extreme temperatures (-5°C to 45°C)
- ✓ Low battery warnings (5%)
- ✓ Invalid messages (should be filtered)
- ✓ Bridge messages (should be ignored)

## Example 4: Monitoring in Real-Time

```bash
# Start the monitor
./scripts/monitor.sh
# Choose option 1 (All MQTT messages)
```

In another terminal:
```bash
# Send some test data
./scripts/quick-test.sh garage_sensor -2.0
./scripts/quick-test.sh living_room 22.5
./scripts/quick-test.sh bedroom 19.0
```

**Monitor output:**
```
zigbee2mqtt/garage_sensor {"temperature": -2.0, "humidity": 65.2, "battery": 85}
zigbee2mqtt/living_room {"temperature": 22.5, "humidity": 58.3, "battery": 92}
zigbee2mqtt/bedroom {"temperature": 19.0, "humidity": 61.7, "battery": 78}
```

## Example 5: Load Testing

Simulate high-frequency updates from 20 sensors:

```bash
./scripts/simulate_sensors.py --sensors 20 --interval 1
```

In another terminal, monitor system resources:
```bash
docker stats home-assistant-rs
```

**Expected output:**
```
CONTAINER          CPU %    MEM USAGE / LIMIT     MEM %
home-assistant-rs  2.5%     45MiB / 7.6GiB        0.58%
```

Check database growth:
```bash
watch -n 1 'ls -lh data/home_assistant.db'
```

## Example 6: Debugging Connection Issues

If messages aren't getting through:

```bash
# Test 1: Can you reach Mosquitto?
mosquitto_pub -h localhost -p 1883 -t "test" -m "hello"

# Test 2: Is the app subscribed?
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/#" -v &
./scripts/quick-test.sh
# You should see the message appear

# Test 3: Check the app's network settings
docker inspect home-assistant-rs | jq '.[0].NetworkSettings'

# Test 4: Try publishing from inside the container
docker exec -it home-assistant-rs sh
# Then: ping 172.17.0.1
```

## Example 7: API Testing

```bash
# Get all latest readings
curl http://localhost:8082/api/temperature/latest

# Get specific sensor
curl http://localhost:8082/api/temperature/sensor/bedroom_temp

# Get history (last hour)
curl http://localhost:8082/api/temperature/history?hours=1

# Get history (last 24 hours)
curl http://localhost:8082/api/temperature/history?hours=24

# Pretty print with jq
curl -s http://localhost:8082/api/temperature/latest | jq '.[] | {sensor: .sensor_id, temp: .temperature, battery: .battery}'
```

## Example 8: Custom Sensor Simulation

Create a custom test for a specific scenario:

```bash
# Simulate a freezer alarm (temperature rising)
for temp in {-18..-5}; do
    echo "Freezer temperature: ${temp}°C"
    mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/garage_freezer" -m "{\"temperature\": $temp, \"battery\": 45}"
    sleep 2
done
```

## Example 9: Batch Testing

Test all sensors in your house at once:

```bash
#!/bin/bash
# Save as: test-all-sensors.sh

SENSORS=(
    "living_room:22.0"
    "bedroom:19.5"
    "kitchen:23.0"
    "bathroom:21.0"
    "office:20.5"
    "garage:-2.0"
    "attic:15.0"
)

for entry in "${SENSORS[@]}"; do
    IFS=':' read -r sensor temp <<< "$entry"
    ./scripts/quick-test.sh "$sensor" "$temp"
    sleep 1
done

echo "✓ All sensors tested"
curl -s http://localhost:8082/api/temperature/latest | jq -r '.[] | "\(.sensor_id): \(.temperature)°C"'
```

## Example 10: Integration with Real Zigbee2MQTT

Once you have real sensors paired with Zigbee2MQTT:

```bash
# Monitor what your real sensors are publishing
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/#" -v | grep -v bridge

# You'll see output like:
# zigbee2mqtt/0x00158d0001a2b3c4 {"temperature":21.5,"humidity":58.2,"battery":100,"linkquality":120}

# Your Rust app automatically processes these messages
# Check the app logs:
docker logs -f home-assistant-rs

# Verify in API:
curl http://localhost:8082/api/temperature/sensor/0x00158d0001a2b3c4
```

## Troubleshooting Examples

### Problem: No data in API

```bash
# 1. Is the app running?
docker ps | grep home-assistant-rs

# 2. Send a test message and watch logs
docker logs -f home-assistant-rs &
./scripts/quick-test.sh

# 3. Check if MQTT is working
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/#" -v

# 4. Verify database file exists
ls -la data/
```

### Problem: Messages not being received

```bash
# Check the app's environment variables
docker inspect home-assistant-rs | jq '.[0].Config.Env'

# Verify MQTT_BROKER is correct
# Should be: 172.17.0.1 (Docker bridge) or your Mosquitto IP

# Test MQTT connectivity from host
mosquitto_pub -h 172.17.0.1 -p 1883 -t "test" -m "hello"
```

### Problem: High memory usage

```bash
# Check database size
du -h data/home_assistant.db

# Check number of readings
sqlite3 data/home_assistant.db "SELECT COUNT(*) FROM temperature_readings;"

# Clean old data (if needed)
# Implement in your app or manually:
sqlite3 data/home_assistant.db "DELETE FROM temperature_readings WHERE timestamp < datetime('now', '-7 days');"
```

## Next Steps

After successful testing:

1. **Add Alerting**: Implement notifications for low battery or temperature thresholds
2. **Add Visualization**: Create graphs of temperature over time
3. **Add Automation**: Trigger actions based on sensor readings
4. **Add More Sensors**: Motion, door/window, light level, etc.
5. **Add Authentication**: Secure your API with tokens or basic auth

See [TESTING.md](../TESTING.md) for more comprehensive testing strategies.
