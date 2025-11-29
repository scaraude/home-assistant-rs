# Deployment Cheatsheet

Quick reference for common deployment tasks.

## Initial Setup (One-Time)

```bash
# 1. Configure your Pi connection
cp .env.deploy.example .env.deploy
vim .env.deploy  # Edit PI_HOST, PI_USER, etc.

# 2. Set up cross-compilation
make setup-cross-compile

# 3. Deploy everything
make deploy-full
```

## Daily Development

```bash
# After making code changes
make quick-deploy

# Watch logs
make logs

# Check if everything is running
make status
```

## Common Tasks

```bash
# Rebuild and deploy
make build transfer-binary restart

# Update configs only
make generate-configs transfer-configs restart

# Restart services
make restart

# Stop all services
make stop

# Start all services
make start
```

## Monitoring

```bash
# All logs
make logs

# Specific service logs
make logs-home-assistant
make logs-mosquitto
make logs-zigbee2mqtt

# Service status
make status

# Pi system info
make pi-info
```

## Backup & Restore

```bash
# Create backup
make backup

# Restore from backup
make restore BACKUP_FILE=backups/backup-20250129-143022.tar.gz
```

## Troubleshooting

```bash
# Test SSH connection
make check-ssh

# Test MQTT broker
make test-connection

# SSH into Pi
make pi-shell

# View system info
make pi-info

# Reinstall dependencies
make install-deps
```

## Service Management on Pi

If you SSH into the Pi (`make pi-shell`):

```bash
# Check status
sudo systemctl status home-assistant-rs
sudo systemctl status mosquitto
sudo systemctl status zigbee2mqtt

# Restart specific service
sudo systemctl restart home-assistant-rs

# View logs
sudo journalctl -u home-assistant-rs -f

# Enable/disable service
sudo systemctl enable home-assistant-rs
sudo systemctl disable home-assistant-rs
```

## USB Zigbee Device

```bash
# Find Zigbee adapter
ssh pi@raspberrypi.local
ls -l /dev/ttyUSB* /dev/ttyACM*

# Check permissions
groups  # Should include 'dialout'

# Add to dialout group if needed
sudo usermod -a -G dialout pi
sudo reboot
```

## Configuration Files

Generated configs are in `configs/`:
- `mosquitto.conf` - Mosquitto MQTT broker config
- `zigbee2mqtt-config.yaml` - Zigbee2MQTT config
- `pi.env` - Application environment variables

Systemd services are in `systemd/`:
- `home-assistant-rs.service`
- `mosquitto.service`
- `zigbee2mqtt.service`

## URLs After Deployment

- Home Assistant RS: `http://raspberrypi.local:8080`
- Zigbee2MQTT UI: `http://raspberrypi.local:8081`

## Full Command Reference

Run `make help` for complete list of available commands.
