#!/bin/bash
# Quick single-message test for home-assistant-rs
# Usage: ./quick-test.sh [sensor_id] [temperature]

CONTAINER_NAME="${MOSQUITTO_CONTAINER:-mosquitto}"
SENSOR_ID="${1:-test_sensor}"
TEMP="${2:-22.5}"
HUMIDITY=$(awk -v min=50 -v max=70 'BEGIN{srand(); print min+rand()*(max-min)}')
BATTERY=$((RANDOM % 30 + 70))

# Check if mosquitto container is running
if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "❌ Mosquitto container '${CONTAINER_NAME}' is not running!"
    echo ""
    echo "Start it with: docker-compose up -d mosquitto"
    echo ""
    exit 1
fi

echo "Publishing test message..."
echo "  Sensor: $SENSOR_ID"
echo "  Temperature: ${TEMP}°C"
echo "  Humidity: $(printf "%.1f" $HUMIDITY)%"
echo "  Battery: ${BATTERY}%"
echo

# Use docker exec to run mosquitto_pub inside the container
docker exec "$CONTAINER_NAME" mosquitto_pub -h localhost -t "zigbee2mqtt/$SENSOR_ID" -m "{
  \"temperature\": $TEMP,
  \"humidity\": $HUMIDITY,
  \"battery\": $BATTERY,
  \"linkquality\": 120
}"

echo "✓ Message published to zigbee2mqtt/$SENSOR_ID"
echo
echo "Check logs: docker logs -f home-assistant-rs"
echo "Query API:  curl http://localhost:8082/api/temperature/latest"
