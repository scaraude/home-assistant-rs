#!/bin/bash
# Quick single-message test for home-assistant-rs
# Usage: ./quick-test.sh [sensor_id] [temperature]

BROKER="${MQTT_BROKER:-localhost}"
PORT="${MQTT_PORT:-1883}"
SENSOR_ID="${1:-test_sensor}"
TEMP="${2:-22.5}"
HUMIDITY=$(awk -v min=50 -v max=70 'BEGIN{srand(); print min+rand()*(max-min)}')
BATTERY=$((RANDOM % 30 + 70))

echo "Publishing test message..."
echo "  Sensor: $SENSOR_ID"
echo "  Temperature: ${TEMP}°C"
echo "  Humidity: $(printf "%.1f" $HUMIDITY)%"
echo "  Battery: ${BATTERY}%"
echo

mosquitto_pub -h "$BROKER" -p "$PORT" -t "zigbee2mqtt/$SENSOR_ID" -m "{
  \"temperature\": $TEMP,
  \"humidity\": $HUMIDITY,
  \"battery\": $BATTERY,
  \"linkquality\": 120
}"

echo "✓ Message published to zigbee2mqtt/$SENSOR_ID"
echo
echo "Check logs: docker logs -f home-assistant-rs"
echo "Query API:  curl http://localhost:8082/api/temperature/latest"
