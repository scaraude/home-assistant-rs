#!/bin/bash
# Quick MQTT testing script for home-assistant-rs
# Sends a variety of test messages to verify your system is working

set -e

# Configuration
BROKER="${MQTT_BROKER:-localhost}"
PORT="${MQTT_PORT:-1883}"
BASE_TOPIC="zigbee2mqtt"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Home Assistant RS - MQTT Test Suite"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Broker: $BROKER:$PORT"
echo

# Check if mosquitto_pub is available
if ! command -v mosquitto_pub &> /dev/null; then
    echo "ERROR: mosquitto_pub not found"
    echo "Install with: sudo apt-get install mosquitto-clients"
    exit 1
fi

echo "✓ mosquitto_pub found"
echo

# Test 1: Basic temperature reading
echo "Test 1: Basic temperature + humidity + battery"
mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/test_sensor_1" -m '{
  "temperature": 22.5,
  "humidity": 65.0,
  "battery": 100,
  "linkquality": 120
}'
echo "  → Published to $BASE_TOPIC/test_sensor_1"
sleep 1

# Test 2: Temperature only (minimal)
echo
echo "Test 2: Temperature only (no humidity/battery)"
mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/minimal_sensor" -m '{
  "temperature": 21.0
}'
echo "  → Published to $BASE_TOPIC/minimal_sensor"
sleep 1

# Test 3: Multiple sensors
echo
echo "Test 3: Multiple sensors with different values"
SENSORS=("living_room" "bedroom" "kitchen" "bathroom")
TEMPS=(22.3 19.8 23.5 21.1)

for i in "${!SENSORS[@]}"; do
    SENSOR="${SENSORS[$i]}"
    TEMP="${TEMPS[$i]}"
    HUMIDITY=$(awk -v min=50 -v max=70 'BEGIN{srand(); print min+rand()*(max-min)}')
    BATTERY=$((RANDOM % 30 + 70))  # Random between 70-100

    mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/$SENSOR" -m "{
  \"temperature\": $TEMP,
  \"humidity\": $HUMIDITY,
  \"battery\": $BATTERY
}"
    echo "  → $SENSOR: ${TEMP}°C, ${HUMIDITY}% humidity, ${BATTERY}% battery"
    sleep 0.5
done

# Test 4: Extreme values
echo
echo "Test 4: Extreme values"
mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/hot_sensor" -m '{
  "temperature": 45.0,
  "humidity": 20.0,
  "battery": 50
}'
echo "  → Hot sensor: 45.0°C"
sleep 0.5

mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/cold_sensor" -m '{
  "temperature": -5.0,
  "humidity": 90.0,
  "battery": 15
}'
echo "  → Cold sensor: -5.0°C"
sleep 0.5

# Test 5: Low battery scenario
echo
echo "Test 5: Low battery warning"
mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/dying_sensor" -m '{
  "temperature": 20.5,
  "humidity": 55.0,
  "battery": 5
}'
echo "  → Sensor with 5% battery"
sleep 1

# Test 6: Invalid messages (should be ignored)
echo
echo "Test 6: Invalid messages (should be ignored by app)"
mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/bridge/info" -m '{
  "version": "1.0.0"
}'
echo "  → Bridge message (should be filtered out)"
sleep 0.5

mosquitto_pub -h "$BROKER" -p "$PORT" -t "$BASE_TOPIC/no_temp_sensor" -m '{
  "humidity": 60.0,
  "battery": 80
}'
echo "  → Message without temperature (should be ignored)"
sleep 1

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Tests completed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "Next steps:"
echo "  1. Check logs: docker logs home-assistant-rs"
echo "  2. Query API: curl http://localhost:8082/api/temperature/latest"
echo "  3. Monitor MQTT: mosquitto_sub -h $BROKER -p $PORT -t 'zigbee2mqtt/#' -v"
echo
