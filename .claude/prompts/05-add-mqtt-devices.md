# MQTT Device Integration

Der core-Crate hat schon rumqttc. Erweitere um echte Geräte.

1. Zigbee2MQTT Topic-Parser (zigbee2mqtt/+/+)
2. Tuya-MQTT Bridge (tuya-mqtt)
3. Shelly MQTT (native)
4. Device-State in SQLite persistieren
5. Event-Bus Notifications bei State-Changes
6. WebSocket Push an Frontend

Config: configs/templates/zigbee2mqtt-config.yaml ist schon da.
Testen: Mosquitto lokal starten, Fake-Devices publishen.
