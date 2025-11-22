#!/bin/bash
# Monitor MQTT traffic and your home-assistant-rs system
# This script helps you see what's happening in real-time

CONTAINER_NAME="${MOSQUITTO_CONTAINER:-mosquitto}"

# Check if mosquitto container is running
if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "❌ Mosquitto container '${CONTAINER_NAME}' is not running!"
    echo ""
    echo "Start it with: docker-compose up -d mosquitto"
    echo ""
    exit 1
fi

# Define mosquitto_sub wrapper to use docker exec
mqtt_sub() {
    docker exec "$CONTAINER_NAME" mosquitto_sub -h localhost "$@"
}

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Home Assistant RS - System Monitor"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

echo "What would you like to monitor?"
echo
echo "  1) All MQTT messages (zigbee2mqtt/#)"
echo "  2) Sensor messages only (excludes bridge)"
echo "  3) Specific sensor"
echo "  4) Application logs (docker)"
echo "  5) Both MQTT + App logs (split screen)"
echo
read -p "Choose [1-5]: " choice

case $choice in
    1)
        echo
        echo "Monitoring all zigbee2mqtt topics..."
        echo "Press Ctrl+C to stop"
        echo
        mqtt_sub -t "zigbee2mqtt/#" -v
        ;;
    2)
        echo
        echo "Monitoring sensor messages (filtering out bridge topics)..."
        echo "Press Ctrl+C to stop"
        echo
        mqtt_sub -t "zigbee2mqtt/#" -v | grep -v "zigbee2mqtt/bridge"
        ;;
    3)
        read -p "Enter sensor ID: " sensor_id
        echo
        echo "Monitoring zigbee2mqtt/$sensor_id..."
        echo "Press Ctrl+C to stop"
        echo
        mqtt_sub -t "zigbee2mqtt/$sensor_id" -v
        ;;
    4)
        echo
        echo "Showing application logs..."
        echo "Press Ctrl+C to stop"
        echo
        docker logs -f home-assistant-rs
        ;;
    5)
        echo
        echo "Split monitoring (requires tmux)..."
        if ! command -v tmux &> /dev/null; then
            echo "ERROR: tmux not found"
            echo "Install with: sudo apt-get install tmux"
            echo
            echo "Falling back to sequential display..."
            echo "Starting MQTT monitor in background..."
            mqtt_sub -t "zigbee2mqtt/#" -v &
            MQTT_PID=$!
            echo "Showing app logs (Ctrl+C to stop)..."
            docker logs -f home-assistant-rs
            kill $MQTT_PID 2>/dev/null
        else
            # Create a tmux session with split panes
            tmux new-session -d -s ha_monitor
            tmux split-window -h -t ha_monitor
            tmux send-keys -t ha_monitor:0.0 "docker exec $CONTAINER_NAME mosquitto_sub -h localhost -t 'zigbee2mqtt/#' -v" C-m
            tmux send-keys -t ha_monitor:0.1 "docker logs -f home-assistant-rs" C-m
            tmux attach-session -t ha_monitor
        fi
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac
