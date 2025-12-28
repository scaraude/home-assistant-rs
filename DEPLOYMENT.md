# Deployment Guide for Raspberry Pi

This guide covers deploying Home Automation RS to a Raspberry Pi without Docker, using native systemd services.

## Prerequisites

### On Your Development Machine

1. **Rust toolchain** with cross-compilation support
2. **SSH access** to your Raspberry Pi
3. **Make** (usually pre-installed on macOS/Linux)

### On Your Raspberry Pi

- Raspberry Pi 3, 4, or newer (tested on Pi 4)
- Raspberry Pi OS (Debian-based)
- SSH enabled
- Internet connection
- Zigbee USB adapter plugged in

## Quick Start

### 1. Configure Deployment Settings

Copy the example deployment configuration:

```bash
cp .env.deploy.example .env.deploy
```

Edit `.env.deploy` with your Raspberry Pi details:

```bash
PI_USER=pi
PI_HOST=raspberrypi.local  # or use IP like 192.168.1.100
SSH_KEY=~/.ssh/id_rsa
ZIGBEE_DEVICE=/dev/ttyUSB0
```

### 2. Set Up Cross-Compilation

Install the Rust cross-compilation toolchain:

```bash
make setup-cross-compile
```

### 3. Deploy Everything

Run the full deployment (this will take 10-15 minutes on first run):

```bash
make deploy-full
```

This single command will:

- ✓ Check SSH connection
- ✓ Generate configuration files
- ✓ Install system dependencies (Mosquitto, Node.js)
- ✓ Create necessary directories
- ✓ Build the Rust binary for ARM
- ✓ Transfer binary and configs to Pi
- ✓ Install Zigbee2MQTT
- ✓ Set up systemd services
- ✓ Start all services

### 4. Verify Deployment

Check service status:

```bash
make status
```

View logs:

```bash
make logs
```

## Step-by-Step Deployment

If you prefer to run steps individually or troubleshoot:

### 1. Test Connection

```bash
make check-ssh
```

### 2. Install Dependencies on Pi

```bash
make install-deps
```

This installs:

- Mosquitto MQTT broker
- Node.js 18.x (for Zigbee2MQTT)
- Build essentials

### 3. Build the Application

```bash
make build
```

Or use `cross-rs` for easier cross-compilation:

```bash
make build-cross
```

### 4. Transfer Files

```bash
make transfer-binary
make transfer-configs
make transfer-frontend
```

### 5. Install Zigbee2MQTT

```bash
make install-zigbee2mqtt
```

### 6. Set Up Services

```bash
make setup-services
make start
```

## Available Make Commands

Run `make help` to see all available commands:

### Deployment Commands

- `make deploy-full` - Full deployment from scratch
- `make quick-deploy` - Rebuild and restart (for quick iterations)
- `make build` - Build the Rust binary
- `make transfer-binary` - Transfer binary to Pi
- `make transfer-configs` - Transfer configuration files
- `make transfer-frontend` - Build and transfer frontend

### Service Management

- `make start` - Start all services
- `make stop` - Stop all services
- `make restart` - Restart all services
- `make status` - Show service status

### Monitoring

- `make logs` - Tail all service logs
- `make logs-home-automation` - Tail Home Automation RS logs only
- `make logs-mosquitto` - Tail Mosquitto logs only
- `make logs-zigbee2mqtt` - Tail Zigbee2MQTT logs only

### Utilities

- `make backup` - Backup database and configs
- `make restore BACKUP_FILE=backups/xxx.tar.gz` - Restore from backup
- `make pi-shell` - SSH into the Pi
- `make pi-info` - Show Pi system information
- `make test-connection` - Test MQTT broker

### Cleanup

- `make clean` - Clean local build artifacts
- `make clean-pi` - Remove all files from Pi (destructive!)

## Development Workflow

### Making Code Changes

After editing code:

```bash
make quick-deploy
```

This rebuilds, transfers the new binary, and restarts the service.

### Checking Logs

To watch logs in real-time:

```bash
make logs
```

Or for a specific service:

```bash
make logs-home-automation
```

### Testing MQTT

Test that Mosquitto is working:

```bash
make test-connection
```

## Architecture

### Services

The deployment sets up three systemd services:

1. **mosquitto.service** - MQTT broker

   - Runs as `mosquitto` user
   - Listens on port 1883
   - Data: `/var/lib/home-automation-rs/mosquitto/`

2. **zigbee2mqtt.service** - Zigbee bridge

   - Runs as `pi` user (needs USB access)
   - Web UI on port 8081
   - Config: `/var/lib/home-automation-rs/zigbee2mqtt/`

3. **home-automation-rs.service** - Your Rust application
   - Runs as `pi` user
   - Web UI on port 8080
   - Binary: `/opt/home-automation-rs/bin/home-automation-rs`

### Directory Structure on Pi

```
/opt/home-automation-rs/          # Application directory
├── bin/
│   └── home-automation-rs        # Rust binary
├── static/                      # Frontend files
└── .env                         # Environment variables

/var/lib/home-automation-rs/      # Data directory
├── database/
│   └── home_automation.db       # SQLite database
├── mosquitto/
│   ├── data/                    # Mosquitto persistence
│   └── log/                     # Mosquitto logs
└── zigbee2mqtt/
    ├── configuration.yaml       # Zigbee2MQTT config
    └── database.db             # Zigbee2MQTT database

/var/log/home-automation-rs/      # Log directory
├── home-automation-rs.log
├── zigbee2mqtt.log
└── mosquitto.log

/etc/mosquitto/
└── mosquitto.conf              # Mosquitto configuration
```

## Troubleshooting

### SSH Connection Issues

If `make check-ssh` fails:

1. Verify Pi is on the network: `ping raspberrypi.local`
2. Check SSH is enabled on Pi
3. Verify SSH key: `ssh-copy-id pi@raspberrypi.local`
4. Try using IP address instead of hostname in `.env.deploy`

### Build Failures

If cross-compilation fails:

1. Install target: `rustup target add armv7-unknown-linux-gnueabihf`
2. Try using cross-rs: `make build-cross`
3. Check you have correct target in `.env.deploy`

### Service Won't Start

Check service status:

```bash
make status
```

View detailed logs:

```bash
ssh pi@raspberrypi.local
sudo journalctl -u home-automation-rs.service -n 50
```

### Zigbee Adapter Not Found

List USB devices:

```bash
make pi-info
```

Find the correct device:

```bash
ssh pi@raspberrypi.local
ls -l /dev/ttyUSB* /dev/ttyACM*
```

Update `ZIGBEE_DEVICE` in `.env.deploy` and redeploy configs:

```bash
make transfer-configs
make restart
```

### Permission Denied on USB Device

Add pi user to dialout group:

```bash
ssh pi@raspberrypi.local
sudo usermod -a -G dialout pi
sudo reboot
```

## Backup and Restore

### Create Backup

```bash
make backup
```

Backups are saved to `backups/backup-YYYYMMDD-HHMMSS.tar.gz`

### Restore from Backup

```bash
make restore BACKUP_FILE=backups/backup-20250129-143022.tar.gz
```

This will:

1. Stop all services
2. Restore database and configurations
3. Restart services

## Configuration Files

### Mosquitto (`configs/mosquitto.conf`)

Generated automatically, but you can customize:

```conf
listener 1883
allow_anonymous true
persistence true
persistence_location /var/lib/home-automation-rs/mosquitto/data/
log_dest file /var/lib/home-automation-rs/mosquitto/log/mosquitto.log
```

### Zigbee2MQTT (`configs/zigbee2mqtt-config.yaml`)

Generated automatically:

```yaml
mqtt:
  server: mqtt://localhost:1883
serial:
  port: /dev/ttyUSB0
frontend:
  port: 8081
data_path: /var/lib/home-automation-rs/zigbee2mqtt
```

### Environment (`configs/pi.env`)

Application environment variables:

```bash
MQTT_BROKER=localhost
MQTT_PORT=1883
HTTP_ADDR=0.0.0.0:8082
DB_PATH=/var/lib/home-automation-rs/database/home_automation.db
```

## Performance Optimization

The `Cargo.toml` is already optimized for embedded systems:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link Time Optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
panic = "abort"     # Reduce binary size
```

This produces a small, efficient binary suitable for Raspberry Pi.

## Security Considerations

### Current Setup (Development)

- Mosquitto allows anonymous connections
- No TLS/SSL encryption
- Services run as `pi` user (except Mosquitto)

### Production Recommendations

1. **Enable Mosquitto authentication**:

   ```bash
   mosquitto_passwd -c /etc/mosquitto/passwd username
   ```

2. **Use TLS for MQTT**:

   - Generate certificates
   - Update Mosquitto config
   - Update client configs

3. **Firewall configuration**:

   ```bash
   sudo ufw allow 8080/tcp  # Home Automation RS
   sudo ufw allow 8081/tcp  # Zigbee2MQTT (optional)
   sudo ufw enable
   ```

4. **Run services as dedicated users** (already done for Mosquitto)

## Next Steps

- Access Home Automation RS: `http://raspberrypi.local:8080`
- Access Zigbee2MQTT UI: `http://raspberrypi.local:8081`
- Pair Zigbee devices through Zigbee2MQTT interface
- Monitor with `make logs`

## Additional Resources

- [Zigbee2MQTT Documentation](https://www.zigbee2mqtt.io/)
- [Mosquitto Documentation](https://mosquitto.org/documentation/)
- [Raspberry Pi Documentation](https://www.raspberrypi.org/documentation/)
