#!/bin/bash
# System health check for home-assistant-rs
# Verifies all components are working correctly

set -e

BROKER="${MQTT_BROKER:-localhost}"
PORT="${MQTT_PORT:-1883}"
API_URL="${API_URL:-http://localhost:8082}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Home Assistant RS - System Health Check"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

PASS=0
FAIL=0

# Function to check status
check() {
    local name="$1"
    local cmd="$2"
    printf "%-50s " "Checking $name..."

    if eval "$cmd" &> /dev/null; then
        echo "✓ PASS"
        ((PASS++))
        return 0
    else
        echo "✗ FAIL"
        ((FAIL++))
        return 1
    fi
}

check_output() {
    local name="$1"
    local cmd="$2"
    local expected="$3"
    printf "%-50s " "Checking $name..."

    output=$(eval "$cmd" 2>&1)
    if [[ "$output" == *"$expected"* ]]; then
        echo "✓ PASS"
        ((PASS++))
        return 0
    else
        echo "✗ FAIL (expected: $expected)"
        ((FAIL++))
        return 1
    fi
}

# Docker checks
echo "▶ Docker Environment"
check "Docker daemon" "docker info"
check "home-assistant-rs container" "docker ps | grep -q home-assistant-rs"
check "Container is healthy" "docker inspect home-assistant-rs | grep -q '\"Status\": \"running\"'"
echo

# MQTT checks
echo "▶ MQTT Broker"
check "Mosquitto clients installed" "command -v mosquitto_pub"
check "MQTT broker reachable" "timeout 2 mosquitto_pub -h $BROKER -p $PORT -t test -m test"
echo

# Network checks
echo "▶ Network Connectivity"
if docker ps | grep -q home-assistant-rs; then
    CONTAINER_IP=$(docker inspect -f '{{range.NetworkSettings.Networks}}{{.IPAddress}}{{end}}' home-assistant-rs)
    echo "  Container IP: $CONTAINER_IP"
    check "Container network" "test -n '$CONTAINER_IP'"
    check "Can ping Docker bridge" "timeout 2 ping -c 1 172.17.0.1"
fi
echo

# API checks
echo "▶ API Endpoints"
check "API is responding" "curl -s -o /dev/null -w '%{http_code}' $API_URL/api/temperature/latest | grep -q 200"
check "API returns JSON" "curl -s $API_URL/api/temperature/latest | jq -e . > /dev/null"
echo

# Database checks
echo "▶ Database"
if [ -f "data/home_assistant.db" ]; then
    check "Database file exists" "test -f data/home_assistant.db"
    check "Database is readable" "sqlite3 data/home_assistant.db 'SELECT 1;'"

    # Count readings
    count=$(sqlite3 data/home_assistant.db "SELECT COUNT(*) FROM temperature_readings;" 2>/dev/null || echo "0")
    echo "  Total readings in database: $count"
else
    echo "  ✗ Database file not found at data/home_assistant.db"
    ((FAIL++))
fi
echo

# Functional test
echo "▶ Functional Test"
printf "%-50s " "Sending test message..."
TEST_SENSOR="health_check_$(date +%s)"
TEST_TEMP="21.5"

if mosquitto_pub -h "$BROKER" -p "$PORT" -t "zigbee2mqtt/$TEST_SENSOR" -m "{\"temperature\": $TEST_TEMP}" 2>/dev/null; then
    echo "✓ Sent"
    ((PASS++))

    # Wait for processing
    sleep 2

    printf "%-50s " "Verifying message was processed..."
    if curl -s "$API_URL/api/temperature/sensor/$TEST_SENSOR" | jq -e ".temperature == $TEST_TEMP" &>/dev/null; then
        echo "✓ PASS"
        ((PASS++))
    else
        echo "✗ FAIL (message not found in API)"
        ((FAIL++))
    fi
else
    echo "✗ FAIL (couldn't send message)"
    ((FAIL++))
fi
echo

# Application logs
echo "▶ Application Status"
printf "%-50s " "Checking for errors in logs..."
ERROR_COUNT=$(docker logs --tail 50 home-assistant-rs 2>&1 | grep -ic "error" || echo "0")
if [ "$ERROR_COUNT" -eq 0 ]; then
    echo "✓ PASS (no errors)"
    ((PASS++))
else
    echo "⚠ WARNING ($ERROR_COUNT errors found)"
    echo "  Last 5 log lines:"
    docker logs --tail 5 home-assistant-rs 2>&1 | sed 's/^/    /'
fi
echo

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
echo

if [ $FAIL -eq 0 ]; then
    echo "  ✓ All checks passed! System is healthy."
    echo
    exit 0
else
    echo "  ✗ Some checks failed. Please review the output above."
    echo
    echo "Common fixes:"
    echo "  • Container not running: docker-compose up -d"
    echo "  • MQTT issues: systemctl restart mosquitto"
    echo "  • Database issues: check file permissions in data/"
    echo "  • API issues: check docker logs home-assistant-rs"
    echo
    exit 1
fi
