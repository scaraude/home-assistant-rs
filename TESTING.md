# End-to-End Testing Guide

This guide explains how to test your Home Assistant Rust system on the Raspberry Pi with simulated sensor inputs.

## Architecture Overview

Your stack consists of:
- **Zigbee2MQTT**: Bridges Zigbee devices to MQTT
- **Mosquitto**: MQTT broker
- **home-assistant-rs**: Your Rust application (subscribes to `zigbee2mqtt/#`)

## Testing Approaches

### 1. Manual Testing with mosquitto_pub

The simplest way to test is publishing MQTT messages directly to Mosquitto.

#### Install mosquitto-clients (if not already installed)
```bash
sudo apt-get install mosquitto-clients
```

#### Publish a test temperature reading
```bash
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/test_sensor" -m '{
  "temperature": 22.5,
  "humidity": 65.0,
  "battery": 100,
  "linkquality": 120
}'
```

#### Test with different sensor IDs
```bash
# Living room sensor
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/living_room_temp" -m '{
  "temperature": 21.3,
  "humidity": 58.2,
  "battery": 95
}'

# Bedroom sensor
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/bedroom_temp" -m '{
  "temperature": 19.8,
  "humidity": 62.5,
  "battery": 87
}'
```

#### Test temperature-only reading (no humidity)
```bash
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/simple_sensor" -m '{
  "temperature": 23.1
}'
```

### 2. Automated Testing with Python Script

For continuous or automated testing, use the provided Python script.

#### Install dependencies
```bash
pip3 install paho-mqtt
```

#### Run the simulator
```bash
python3 scripts/simulate_sensors.py
```

This will:
- Simulate 3 sensors with realistic temperature variations
- Publish readings every 5 seconds
- Include random humidity and battery drain
- Run continuously until stopped (Ctrl+C)

#### Customize sensor behavior
Edit `scripts/simulate_sensors.py` to adjust:
- Number of sensors
- Update interval
- Temperature ranges
- Humidity ranges
- Battery drain rate

### 3. Testing with Shell Script

For quick batch testing, use the shell script:

```bash
bash scripts/test_mqtt.sh
```

This sends a series of test messages with different scenarios:
- Normal readings
- Temperature-only readings
- Varying battery levels
- Multiple sensor IDs

### 4. Monitor Your System

#### Watch MQTT traffic
```bash
# Subscribe to all zigbee2mqtt topics
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/#" -v

# Subscribe to specific sensor
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/test_sensor" -v
```

#### Check your Rust app logs
```bash
docker logs -f home-assistant-rs
```

#### Query the API
```bash
# Get latest readings
curl http://localhost:8082/api/temperature/latest

# Get specific sensor
curl http://localhost:8082/api/temperature/sensor/test_sensor

# Get temperature history
curl http://localhost:8082/api/temperature/history?hours=1
```

## Test Scenarios

### Scenario 1: Basic Functionality
1. Start your containers (`docker-compose up -d`)
2. Publish a single test message
3. Check logs for processing
4. Query API to verify storage

### Scenario 2: Multiple Sensors
1. Publish messages from 5 different sensor IDs
2. Verify all sensors appear in API
3. Check database for all readings

### Scenario 3: High Frequency
1. Run the Python simulator with 1-second intervals
2. Monitor system performance
3. Verify no message loss

### Scenario 4: Edge Cases
```bash
# Missing optional fields
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/minimal" -m '{"temperature": 20.0}'

# Very high temperature
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/hot" -m '{"temperature": 45.0}'

# Very low temperature
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/cold" -m '{"temperature": -10.0}'

# Low battery
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/dying" -m '{"temperature": 22.0, "battery": 5}'
```

## Debugging

### Check Mosquitto is running
```bash
systemctl status mosquitto
# or
docker ps | grep mosquitto
```

### Check Zigbee2MQTT is running
```bash
docker ps | grep zigbee2mqtt
```

### Check MQTT connection from container
```bash
docker exec -it home-assistant-rs sh
# Then try to ping the broker
ping 172.17.0.1
```

### Verify topic subscription
Check your Rust app logs for:
```
MQTT error: ...
```

If you see connection errors, verify the `MQTT_BROKER` environment variable matches your Mosquitto host.

## Expected Message Format

Your Rust application expects messages on topics matching `zigbee2mqtt/<sensor_id>` with this JSON structure:

```json
{
  "temperature": 22.5,       // Required (float, Celsius)
  "humidity": 65.0,          // Optional (float, percentage)
  "battery": 100,            // Optional (integer, percentage)
  "linkquality": 120         // Optional (ignored by app)
}
```

**Note**: Messages without a `temperature` field will be ignored.

## Integration with Real Sensors

Once you've verified the system works with simulated data:

1. Pair real Zigbee sensors with Zigbee2MQTT
2. Check Zigbee2MQTT logs to see device topics
3. Update your test scripts to match real sensor IDs
4. Gradually transition from simulated to real sensors

## Troubleshooting

| Issue | Solution |
|-------|----------|
| No messages received | Check MQTT broker connectivity, verify topic subscription |
| Messages not in database | Check Rust app logs for parsing errors |
| Container can't reach broker | Verify network settings, check `172.17.0.1` is correct |
| API returns empty | Ensure messages contain `temperature` field |

## Next Steps

After testing with simulated data:
1. Add alerting for low battery levels
2. Implement temperature threshold notifications
3. Add data visualization
4. Set up automated backups of the database
