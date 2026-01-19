# Configuration Templates

This directory contains template files for systemd services and application configurations.

## Usage

Templates use `{{VARIABLE}}` placeholders that are replaced during generation:

```bash
make generate-service-files  # Generate systemd service files
make generate-configs        # Generate application config files
```

## Available Templates

### Systemd Services
- `home-automation-rs.service` - Main application service
- `mosquitto.service` - MQTT broker service
- `zigbee2mqtt.service` - Zigbee coordinator bridge
- `system-monitor.service` - System metrics logger

### Application Configs
- `mosquitto.conf` - MQTT broker configuration
- `zigbee2mqtt-config.yaml` - Zigbee2MQTT settings
- `pi.env` - Runtime environment variables

## Variables

Common placeholders used in templates:

- `{{PI_USER}}` - SSH user for deployment (default: ludovic)
- `{{DEPLOY_DIR}}` - Application install directory (default: /opt/home-automation-rs)
- `{{DATA_DIR}}` - Runtime data directory (default: /var/lib/home-automation-rs)
- `{{LOG_DIR}}` - Log files directory (default: /var/log/home-automation-rs)
- `{{BINARY_NAME}}` - Compiled binary name (default: home-automation-rs)
- `{{ZIGBEE_DEVICE}}` - Serial port for Zigbee coordinator
- `{{Z2M_PAN_ID}}`, `{{Z2M_EXT_PAN_ID}}`, `{{Z2M_NETWORK_KEY}}`, `{{Z2M_CHANNEL}}` - Zigbee network parameters

Override defaults in `.env.deploy` file.
