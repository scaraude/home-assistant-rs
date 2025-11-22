# Testing Cheat Sheet

Quick reference for testing home-assistant-rs on your Raspberry Pi.

## One-Liners

```bash
# Quick test
./scripts/quick-test.sh

# Quick test with custom sensor
./scripts/quick-test.sh bedroom 21.5

# Run all tests
./scripts/test_mqtt.sh

# Simulate sensors
./scripts/simulate_sensors.py

# Monitor everything
./scripts/monitor.sh

# Watch app logs
docker logs -f home-assistant-rs

# Watch MQTT messages
mosquitto_sub -h localhost -p 1883 -t "zigbee2mqtt/#" -v

# Check latest readings
curl http://localhost:8082/api/temperature/latest | jq

# Restart app
docker-compose restart app
```

## Manual MQTT Publishing

```bash
# Basic reading
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/test" -m '{"temperature": 22.5}'

# Full reading
mosquitto_pub -h localhost -p 1883 -t "zigbee2mqtt/sensor1" -m '{
  "temperature": 22.5,
  "humidity": 65.0,
  "battery": 100
}'
```

## API Endpoints

```bash
# Latest from all sensors
curl http://localhost:8082/api/temperature/latest

# Specific sensor
curl http://localhost:8082/api/temperature/sensor/SENSOR_ID

# History (default: 24h)
curl http://localhost:8082/api/temperature/history

# History (custom)
curl http://localhost:8082/api/temperature/history?hours=6
```

## Debugging

```bash
# App logs
docker logs home-assistant-rs

# App logs (follow)
docker logs -f home-assistant-rs

# App logs (last 50 lines)
docker logs --tail 50 home-assistant-rs

# Container status
docker ps -a | grep home-assistant

# Container inspect
docker inspect home-assistant-rs

# Container stats (resource usage)
docker stats home-assistant-rs

# Check database
ls -lh data/
sqlite3 data/home_assistant.db "SELECT COUNT(*) FROM temperature_readings;"

# Check Mosquitto
systemctl status mosquitto
# or
docker ps | grep mosquitto

# Test MQTT connectivity
mosquitto_pub -h localhost -p 1883 -t "test" -m "hello"
mosquitto_sub -h localhost -p 1883 -t "test"
```

## Useful Commands

```bash
# Restart everything
docker-compose restart

# View all containers
docker-compose ps

# Stop everything
docker-compose down

# Start everything
docker-compose up -d

# View environment variables
docker exec home-assistant-rs env

# Access container shell
docker exec -it home-assistant-rs sh

# Check container network
docker network inspect bridge

# Follow multiple logs
docker-compose logs -f
```

## Database Queries

```bash
# Count readings
sqlite3 data/home_assistant.db "SELECT COUNT(*) FROM temperature_readings;"

# Latest 10 readings
sqlite3 data/home_assistant.db "SELECT * FROM temperature_readings ORDER BY timestamp DESC LIMIT 10;"

# Readings by sensor
sqlite3 data/home_assistant.db "SELECT * FROM temperature_readings WHERE sensor_id = 'test_sensor';"

# Delete old data
sqlite3 data/home_assistant.db "DELETE FROM temperature_readings WHERE timestamp < datetime('now', '-7 days');"

# Database size
du -h data/home_assistant.db
```

## Environment Variables

```bash
# Check current values
echo $MQTT_BROKER
echo $MQTT_PORT

# Set for testing
export MQTT_BROKER=localhost
export MQTT_PORT=1883

# Use in docker-compose
# Edit docker-compose.yml:
environment:
  - MQTT_BROKER=172.17.0.1
  - MQTT_PORT=1883
```

## Common Issues

| Problem | Solution |
|---------|----------|
| No messages received | Check MQTT_BROKER env var |
| Can't connect to API | Check port mapping (8082:8080) |
| Messages not stored | Check logs for JSON parse errors |
| High CPU usage | Check for MQTT connection loop |
| Database locked | Stop app, check file permissions |

## Testing Workflow

```bash
# 1. Start system
docker-compose up -d

# 2. Monitor (terminal 1)
docker logs -f home-assistant-rs

# 3. Send test data (terminal 2)
./scripts/quick-test.sh

# 4. Verify (terminal 3)
curl http://localhost:8082/api/temperature/latest | jq

# 5. Clean up (when done)
docker-compose down
```

## Quick System Check

```bash
# Run this to verify everything is working:
echo "=== Docker Containers ==="
docker ps | grep -E "home-assistant|mosquitto|zigbee"

echo -e "\n=== MQTT Test ==="
timeout 2 mosquitto_sub -h localhost -p 1883 -t "test" &
sleep 0.5
mosquitto_pub -h localhost -p 1883 -t "test" -m "OK"
sleep 1

echo -e "\n=== API Test ==="
curl -s http://localhost:8082/api/temperature/latest | jq -e . > /dev/null && echo "✓ API responding" || echo "✗ API not responding"

echo -e "\n=== Database ==="
ls -lh data/home_assistant.db 2>/dev/null && echo "✓ Database exists" || echo "✗ Database not found"

echo -e "\n=== App Logs (last 5 lines) ==="
docker logs --tail 5 home-assistant-rs
```

## Performance Testing

```bash
# Simulate 10 sensors, 1 message per second
./scripts/simulate_sensors.py --sensors 10 --interval 1 &
SIM_PID=$!

# Monitor for 60 seconds
sleep 60
docker stats --no-stream home-assistant-rs

# Stop simulation
kill $SIM_PID

# Check results
curl -s http://localhost:8082/api/temperature/latest | jq 'length'
```

## Files & Directories

```
├── docker-compose.yml     # Container configuration
├── Dockerfile             # App build config
├── data/
│   └── home_assistant.db  # SQLite database
├── scripts/
│   ├── quick-test.sh      # Single message test
│   ├── test_mqtt.sh       # Full test suite
│   ├── simulate_sensors.py # Continuous simulation
│   └── monitor.sh         # Real-time monitoring
└── TESTING.md             # Full testing guide
```

## Help

```bash
# Script help
./scripts/simulate_sensors.py --help

# Docker help
docker-compose --help
docker logs --help

# MQTT help
mosquitto_pub --help
mosquitto_sub --help
```
