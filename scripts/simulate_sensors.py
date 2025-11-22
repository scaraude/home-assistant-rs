#!/usr/bin/env python3
"""
Simulate multiple Zigbee temperature/humidity sensors publishing to MQTT.
This script helps test your home-assistant-rs system end-to-end.

Usage:
    python3 simulate_sensors.py [--broker HOST] [--port PORT]
"""

from paho.mqtt.client import Client
import json
import time
import random
import argparse
from datetime import datetime


class SensorSimulator:
    """Simulates a Zigbee temperature/humidity sensor"""

    def __init__(self, sensor_id, base_temp=20.0, base_humidity=60.0):
        self.sensor_id = sensor_id
        self.temperature = base_temp
        self.humidity = base_humidity
        self.battery = 100
        self.linkquality = random.randint(80, 150)

    def update(self):
        """Update sensor readings with realistic variations"""
        # Temperature: slow random walk with daily cycle simulation
        temp_change = random.uniform(-0.3, 0.3)
        self.temperature += temp_change
        self.temperature = max(15.0, min(30.0, self.temperature))  # Clamp to realistic range

        # Humidity: moderate variation
        humidity_change = random.uniform(-2.0, 2.0)
        self.humidity += humidity_change
        self.humidity = max(30.0, min(90.0, self.humidity))  # Clamp to realistic range

        # Battery: very slow drain
        if random.random() < 0.01:  # 1% chance per update
            self.battery = max(0, self.battery - 1)

        # Link quality: occasional fluctuation
        if random.random() < 0.1:  # 10% chance per update
            self.linkquality = random.randint(80, 150)

    def get_message(self):
        """Generate a zigbee2mqtt-compatible message"""
        return {
            "temperature": round(self.temperature, 1),
            "humidity": round(self.humidity, 1),
            "battery": self.battery,
            "linkquality": self.linkquality
        }

    def get_topic(self):
        """Get the MQTT topic for this sensor"""
        return f"zigbee2mqtt/{self.sensor_id}"


def on_connect(client, userdata, flags, rc):
    """Callback when connected to MQTT broker"""
    if rc == 0:
        print(f"✓ Connected to MQTT broker")
    else:
        print(f"✗ Connection failed with code {rc}")


def main():
    parser = argparse.ArgumentParser(description="Simulate Zigbee sensors for testing")
    parser.add_argument("--broker", default="localhost", help="MQTT broker host (default: localhost)")
    parser.add_argument("--port", type=int, default=1883, help="MQTT broker port (default: 1883)")
    parser.add_argument("--interval", type=int, default=5, help="Update interval in seconds (default: 5)")
    parser.add_argument("--sensors", type=int, default=3, help="Number of sensors to simulate (default: 3)")
    args = parser.parse_args()

    # Create MQTT client
    client = Client(client_id="sensor_simulator")
    client.on_connect = on_connect

    print(f"Connecting to MQTT broker at {args.broker}:{args.port}...")
    try:
        client.connect(args.broker, args.port, 60)
    except Exception as e:
        print(f"✗ Failed to connect to MQTT broker: {e}")
        return

    client.loop_start()

    # Create simulated sensors with different base values
    sensors = []
    sensor_names = [
        "living_room_temp",
        "bedroom_temp",
        "kitchen_temp",
        "bathroom_temp",
        "office_temp"
    ]

    for i in range(min(args.sensors, len(sensor_names))):
        base_temp = random.uniform(18.0, 24.0)
        base_humidity = random.uniform(50.0, 70.0)
        sensor = SensorSimulator(sensor_names[i], base_temp, base_humidity)
        sensors.append(sensor)
        print(f"✓ Created sensor: {sensor_names[i]}")

    print(f"\nSimulating {len(sensors)} sensors, publishing every {args.interval}s")
    print("Press Ctrl+C to stop\n")

    try:
        while True:
            for sensor in sensors:
                # Update sensor values
                sensor.update()

                # Publish to MQTT
                topic = sensor.get_topic()
                message = sensor.get_message()
                payload = json.dumps(message)

                result = client.publish(topic, payload, qos=0)

                if result.rc == 0:
                    timestamp = datetime.now().strftime("%H:%M:%S")
                    print(f"[{timestamp}] {sensor.sensor_id:20s} → "
                          f"{message['temperature']:5.1f}°C  "
                          f"{message['humidity']:5.1f}%  "
                          f"Battery: {message['battery']:3d}%")
                else:
                    print(f"✗ Failed to publish to {topic}")

            print()  # Empty line between rounds
            time.sleep(args.interval)

    except KeyboardInterrupt:
        print("\n\nStopping simulator...")
        client.loop_stop()
        client.disconnect()
        print("✓ Disconnected from broker")


if __name__ == "__main__":
    main()
