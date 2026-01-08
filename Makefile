.PHONY: help build deploy install-deps setup-services start stop restart status logs clean backup restore

# Configuration - Set these in .env.deploy (see .env.deploy.example)
PI_IP=$(PI_HOST)

# Binary and service names
BINARY_NAME = home-automation-rs
SERVICE_FILES = home-automation-rs.service mosquitto.service zigbee2mqtt.service system-monitor.service

# Colors for output
COLOR_RESET = \033[0m
COLOR_BOLD = \033[1m
COLOR_GREEN = \033[32m
COLOR_YELLOW = \033[33m
COLOR_BLUE = \033[34m

help: ## Show this help message
	@echo "$(COLOR_BOLD)Home Automation RS - Deployment Makefile$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BLUE)Configuration:$(COLOR_RESET)"
	@echo "  PI_USER=$(PI_USER)"
	@echo "  PI_HOST=$(PI_HOST)"
	@echo "  RUST_TARGET=$(RUST_TARGET)"
	@echo "  DEPLOY_DIR=$(DEPLOY_DIR)"
	@echo ""
	@echo "$(COLOR_BLUE)Available targets:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-20s$(COLOR_RESET) %s\n", $$1, $$2}'

# Load environment variables if .env.deploy exists
-include .env.deploy

check-ssh: ## Verify SSH connection to Raspberry Pi
	@echo "$(COLOR_BLUE)Checking SSH connection to $(PI_USER)@$(PI_IP)...$(COLOR_RESET)"
	@ssh -i $(SSH_KEY) -o ConnectTimeout=5 $(PI_USER)@$(PI_IP) "echo '$(COLOR_GREEN)✓ SSH connection successful$(COLOR_RESET)'" || \
		(echo "$(COLOR_YELLOW)✗ SSH connection failed. Please check PI_HOST, PI_USER, and SSH_KEY settings.$(COLOR_RESET)" && exit 1)

setup-cross-compile: ## Install Rust cross-compilation tools
	@echo "$(COLOR_BLUE)Setting up Rust cross-compilation for $(RUST_TARGET)...$(COLOR_RESET)"
	rustup target add $(RUST_TARGET)
	@if ! command -v cross &> /dev/null; then \
		echo "Installing cross-rs for easier cross-compilation..."; \
		cargo install cross --git https://github.com/cross-rs/cross; \
	fi
	@echo "$(COLOR_GREEN)✓ Cross-compilation setup complete$(COLOR_RESET)"

build: ## Build the Rust binary for Raspberry Pi
	@echo "$(COLOR_BLUE)Building $(BINARY_NAME) for $(RUST_TARGET)...$(COLOR_RESET)"
	cross build --release --target=$(RUST_TARGET)
	@ls -lh target/$(RUST_TARGET)/release/$(BINARY_NAME)
	@echo "$(COLOR_GREEN)✓ Build complete$(COLOR_RESET)"

install-deps: check-ssh ## Install dependencies on Raspberry Pi (Mosquitto, Node.js, npm)
	@echo "$(COLOR_BLUE)Installing dependencies on Raspberry Pi...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) 'bash -s' < scripts/install-pi-deps.sh
	@echo "$(COLOR_GREEN)✓ Dependencies installed$(COLOR_RESET)"

setup-directories: check-ssh ## Create necessary directories on Raspberry Pi
	@echo "$(COLOR_BLUE)Creating directories on Raspberry Pi...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo mkdir -p $(DEPLOY_DIR)/bin && \
		sudo mkdir -p $(DEPLOY_DIR)/static && \
		sudo mkdir -p $(DATA_DIR)/database && \
		sudo mkdir -p $(DATA_DIR)/mosquitto/data && \
		sudo mkdir -p $(DATA_DIR)/mosquitto/log && \
		sudo mkdir -p $(DATA_DIR)/zigbee2mqtt && \
		sudo mkdir -p $(LOG_DIR) && \
		sudo mkdir -p /etc/mosquitto && \
		sudo chown -R $(PI_USER):$(PI_USER) $(DEPLOY_DIR) && \
		sudo chown -R $(PI_USER):$(PI_USER) $(DATA_DIR) && \
		sudo chown -R mosquitto:mosquitto $(DATA_DIR)/mosquitto 2>/dev/null || sudo chown -R $(PI_USER):$(PI_USER) $(DATA_DIR)/mosquitto && \
		sudo chown -R $(PI_USER):$(PI_USER) $(LOG_DIR)"
	@echo "$(COLOR_GREEN)✓ Directories created$(COLOR_RESET)"

transfer-binary: build check-ssh setup-directories ## Transfer the compiled binary to Raspberry Pi
	@echo "$(COLOR_BLUE)Transferring binary to Raspberry Pi...$(COLOR_RESET)"
	scp -i $(SSH_KEY) target/$(RUST_TARGET)/release/$(BINARY_NAME) $(PI_USER)@$(PI_IP):$(DEPLOY_DIR)/bin/
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "chmod +x $(DEPLOY_DIR)/bin/$(BINARY_NAME)"
	@echo "$(COLOR_GREEN)✓ Binary transferred$(COLOR_RESET)"

transfer-configs: check-ssh setup-directories ## Transfer configuration files to Raspberry Pi
	@echo "$(COLOR_BLUE)Transferring configuration files...$(COLOR_RESET)"
	# Transfer Mosquitto config
	scp -i $(SSH_KEY) configs/mosquitto.conf $(PI_USER)@$(PI_IP):/tmp/
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "sudo mv /tmp/mosquitto.conf /etc/mosquitto/mosquitto.conf"
	# Transfer Zigbee2MQTT config
	scp -i $(SSH_KEY) configs/zigbee2mqtt-config.yaml $(PI_USER)@$(PI_IP):$(DATA_DIR)/zigbee2mqtt/configuration.yaml
	# Transfer environment file
	scp -i $(SSH_KEY) configs/pi.env $(PI_USER)@$(PI_IP):$(DEPLOY_DIR)/.env
	@echo "$(COLOR_GREEN)✓ Configuration files transferred$(COLOR_RESET)"

transfer-frontend: check-ssh ## Build and transfer frontend to Raspberry Pi
	@echo "$(COLOR_BLUE)Building and transferring frontend...$(COLOR_RESET)"
	@if [ -d "frontend" ]; then \
		cd frontend && npm install && npm run build && cd ..; \
		ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "mkdir -p $(DEPLOY_DIR)/static"; \
		scp -i $(SSH_KEY) -r static/* $(PI_USER)@$(PI_IP):$(DEPLOY_DIR)/static/; \
		echo "$(COLOR_GREEN)✓ Frontend transferred$(COLOR_RESET)"; \
	else \
		echo "$(COLOR_YELLOW)⚠ No frontend directory found, skipping$(COLOR_RESET)"; \
	fi

transfer-monitor: check-ssh ## Transfer monitor.sh script to Raspberry Pi
	@echo "$(COLOR_BLUE)Transferring monitor.sh script...$(COLOR_RESET)"
	scp -i $(SSH_KEY) monitor.sh $(PI_USER)@$(PI_IP):$(DEPLOY_DIR)/
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "chmod +x $(DEPLOY_DIR)/monitor.sh"
	@echo "$(COLOR_GREEN)✓ Monitor script transferred$(COLOR_RESET)"

setup-services: check-ssh ## Install and enable systemd services
	@echo "$(COLOR_BLUE)Setting up systemd services...$(COLOR_RESET)"
	# Generate service files from templates
	$(MAKE) generate-service-files
	# Transfer service files
	scp -i $(SSH_KEY) systemd/*.service $(PI_USER)@$(PI_IP):/tmp/
	# Install and enable services
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo mv /tmp/*.service /etc/systemd/system/ && \
		sudo systemctl daemon-reload && \
		sudo systemctl enable mosquitto.service && \
		sudo systemctl enable zigbee2mqtt.service && \
		sudo systemctl enable home-automation-rs.service && \
		sudo systemctl enable system-monitor.service"
	@echo "$(COLOR_GREEN)✓ Services configured$(COLOR_RESET)"

generate-service-files: ## Generate systemd service files
	@echo "$(COLOR_BLUE)Generating systemd service files...$(COLOR_RESET)"
	@mkdir -p systemd
	# Home Automation RS service
	@echo "[Unit]\n\
Description=Home Automation RS - Rust-based Home Automation\n\
After=mosquitto.service\n\
Requires=mosquitto.service\n\
\n\
[Service]\n\
Type=simple\n\
User=$(PI_USER)\n\
WorkingDirectory=$(DEPLOY_DIR)\n\
EnvironmentFile=$(DEPLOY_DIR)/.env\n\
ExecStart=$(DEPLOY_DIR)/bin/$(BINARY_NAME)\n\
Restart=always\n\
RestartSec=10\n\
StandardOutput=append:$(LOG_DIR)/home-automation-rs.log\n\
StandardError=append:$(LOG_DIR)/home-automation-rs-error.log\n\
\n\
[Install]\n\
WantedBy=multi-user.target" > systemd/home-automation-rs.service
	# Mosquitto service (override default if needed)
	@echo "[Unit]\n\
Description=Mosquitto MQTT Broker\n\
\n\
[Service]\n\
Type=simple\n\
User=mosquitto\n\
ExecStart=/usr/sbin/mosquitto -c /etc/mosquitto/mosquitto.conf\n\
Restart=always\n\
RestartSec=10\n\
\n\
[Install]\n\
WantedBy=multi-user.target" > systemd/mosquitto.service
	# Zigbee2MQTT service
	@echo "[Unit]\n\
Description=Zigbee2MQTT Bridge\n\
After=mosquitto.service\n\
Requires=mosquitto.service\n\
\n\
[Service]\n\
Type=simple\n\
User=$(PI_USER)\n\
WorkingDirectory=$(DATA_DIR)/zigbee2mqtt\n\
ExecStart=/usr/bin/zigbee2mqtt\n\
Restart=always\n\
RestartSec=10\n\
StandardOutput=append:$(LOG_DIR)/zigbee2mqtt.log\n\
StandardError=append:$(LOG_DIR)/zigbee2mqtt-error.log\n\
Environment=\"NODE_ENV=production\"\n\
Environment=\"ZIGBEE2MQTT_DATA=$(DATA_DIR)/zigbee2mqtt\"\n\
\n\
[Install]\n\
WantedBy=multi-user.target" > systemd/zigbee2mqtt.service
	# System Monitor service
	@echo "[Unit]\n\
Description=System Monitor - CPU, RAM and Temperature Logger\n\
After=network.target\n\
\n\
[Service]\n\
Type=simple\n\
User=$(PI_USER)\n\
ExecStart=$(DEPLOY_DIR)/monitor.sh\n\
Restart=always\n\
RestartSec=10\n\
StandardOutput=append:$(LOG_DIR)/monitor.log\n\
StandardError=append:$(LOG_DIR)/monitor-error.log\n\
\n\
[Install]\n\
WantedBy=multi-user.target" > systemd/system-monitor.service
	@echo "$(COLOR_GREEN)✓ Service files generated in systemd/$(COLOR_RESET)"

generate-configs: ## Generate configuration files for deployment
	@echo "$(COLOR_BLUE)Generating configuration files...$(COLOR_RESET)"
	@mkdir -p configs
	# Mosquitto config
	@echo "listener 1883\n\
allow_anonymous true\n\
persistence true\n\
persistence_location $(DATA_DIR)/mosquitto/data/\n\
log_dest file $(DATA_DIR)/mosquitto/log/mosquitto.log\n\
log_dest stdout" > configs/mosquitto.conf
	# Zigbee2MQTT config
	@echo "mqtt:\n\
  server: mqtt://localhost:1883\n\
serial:\n\
  port: $(ZIGBEE_DEVICE)\n\
frontend: false\n\
advanced:\n\
  log_level: info\n\
  log_directory: $(LOG_DIR)\n\
data_path: $(DATA_DIR)/zigbee2mqtt" > configs/zigbee2mqtt-config.yaml
	# Environment file for Raspberry Pi
	@echo "MQTT_BROKER=localhost\n\
MQTT_PORT=1883\n\
HTTP_ADDR=0.0.0.0:8082\n\
DB_PATH=$(DATA_DIR)/database/home_automation.db" > configs/pi.env
	@echo "$(COLOR_GREEN)✓ Configuration files generated in configs/$(COLOR_RESET)"

install-zigbee2mqtt: check-ssh ## Install Zigbee2MQTT on Raspberry Pi
	@echo "$(COLOR_BLUE)Installing Zigbee2MQTT...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		cd $(DATA_DIR)/zigbee2mqtt && \
		sudo npm install -g zigbee2mqtt && \
		zigbee2mqtt --version"
	@echo "$(COLOR_GREEN)✓ Zigbee2MQTT installed$(COLOR_RESET)"

deploy-full: ## Full deployment (build, transfer, configure, and start services)
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)Starting full deployment...$(COLOR_RESET)"
	$(MAKE) check-ssh
	$(MAKE) generate-configs
	$(MAKE) install-deps
	$(MAKE) setup-directories
	$(MAKE) build
	$(MAKE) transfer-binary
	$(MAKE) transfer-configs
	$(MAKE) transfer-frontend
	$(MAKE) transfer-monitor
	$(MAKE) install-zigbee2mqtt
	$(MAKE) setup-services
	$(MAKE) start
	@echo "$(COLOR_BOLD)$(COLOR_GREEN)✓ Full deployment complete!$(COLOR_RESET)"
	$(MAKE) status

deploy: deploy-full ## Alias for deploy-full

quick-deploy:
	@echo "$(COLOR_BLUE)Stopping services...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "sudo systemctl stop home-automation-rs.service"
	@echo "$(COLOR_GREEN)✓ Service stopped$(COLOR_RESET)"
	$(MAKE) build transfer-binary restart-home-automation ## Quick deploy: build, transfer binary, and restart service
	@echo "$(COLOR_GREEN)✓ Quick deployment complete$(COLOR_RESET)"

quick-deploy-frontend: check-ssh transfer-frontend restart-home-automation ## Quick deploy frontend only
	@echo "$(COLOR_GREEN)✓ Frontend quick deployment complete$(COLOR_RESET)"
	
start: check-ssh ## Start all services on Raspberry Pi
	@echo "$(COLOR_BLUE)Starting services...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo systemctl start mosquitto.service && \
		sudo systemctl start zigbee2mqtt.service && \
		sudo systemctl start home-automation-rs.service && \
		sudo systemctl start system-monitor.service"
	@echo "$(COLOR_GREEN)✓ Services started$(COLOR_RESET)"
	@sleep 2
	$(MAKE) status

stop: check-ssh ## Stop all services on Raspberry Pi
	@echo "$(COLOR_BLUE)Stopping services...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo systemctl stop home-automation-rs.service; \
		sudo systemctl stop zigbee2mqtt.service; \
		sudo systemctl stop mosquitto.service; \
		sudo systemctl stop system-monitor.service"
	@echo "$(COLOR_GREEN)✓ Services stopped$(COLOR_RESET)"

restart: check-ssh ## Restart all services on Raspberry Pi
	@echo "$(COLOR_BLUE)Restarting services...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo systemctl restart mosquitto.service && \
		sudo systemctl restart zigbee2mqtt.service && \
		sudo systemctl restart home-automation-rs.service && \
		sudo systemctl restart system-monitor.service"
	@echo "$(COLOR_GREEN)✓ Services restarted$(COLOR_RESET)"
	@sleep 2
	$(MAKE) status

restart-home-automation: check-ssh ## Restart home automation service on Raspberry Pi
	@echo "$(COLOR_BLUE)Restarting services...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo systemctl restart home-automation-rs.service"
	@echo "$(COLOR_GREEN)✓ Services restarted$(COLOR_RESET)"
	@ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		echo '$(COLOR_BOLD)Home Automation RS:$(COLOR_RESET)' && \
		sudo systemctl status home-automation-rs.service --no-pager -l | head -n 10"

status: check-ssh ## Check status of all services
	@echo "$(COLOR_BLUE)Service Status:$(COLOR_RESET)"
	@ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		echo '$(COLOR_BOLD)Mosquitto:$(COLOR_RESET)' && \
		sudo systemctl status mosquitto.service --no-pager -l | head -n 10 && \
		echo '' && \
		echo '$(COLOR_BOLD)Zigbee2MQTT:$(COLOR_RESET)' && \
		sudo systemctl status zigbee2mqtt.service --no-pager -l | head -n 10 && \
		echo '' && \
		echo '$(COLOR_BOLD)Home Automation RS:$(COLOR_RESET)' && \
		sudo systemctl status home-automation-rs.service --no-pager -l | head -n 10 && \
		echo '' && \
		echo '$(COLOR_BOLD)System Monitor:$(COLOR_RESET)' && \
		sudo systemctl status system-monitor.service --no-pager -l | head -n 10"

logs: check-ssh ## Tail all service logs
	@echo "$(COLOR_BLUE)Tailing logs (Ctrl+C to exit)...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo journalctl -f -u mosquitto.service -u zigbee2mqtt.service -u home-automation-rs.service -u system-monitor.service"

logs-home-automation: check-ssh ## Tail Home Automation RS logs only
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "sudo tail -f -n 20 /var/log/home-automation-rs/home-automation-rs.log"

logs-mosquitto: check-ssh ## Tail Mosquitto logs only
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "sudo journalctl -f -u mosquitto.service"

logs-zigbee2mqtt: check-ssh ## Tail Zigbee2MQTT logs only
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "sudo tail -f -n 20 /var/log/home-automation-rs/zigbee2mqtt.log"

logs-monitor: check-ssh ## Tail System Monitor logs only
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "sudo journalctl -f -u system-monitor.service"

backup: check-ssh ## Backup database and configurations from Raspberry Pi
	@echo "$(COLOR_BLUE)Creating backup...$(COLOR_RESET)"
	@mkdir -p backups
	@BACKUP_NAME=backup-$$(date +%Y%m%d-%H%M%S).tar.gz && \
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		cd $(DATA_DIR) && \
		sudo tar -czf /tmp/$$BACKUP_NAME database/ zigbee2mqtt/configuration.yaml zigbee2mqtt/database.db mosquitto/data/" && \
	scp -i $(SSH_KEY) $(PI_USER)@$(PI_IP):/tmp/$$BACKUP_NAME backups/ && \
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "rm /tmp/$$BACKUP_NAME" && \
	echo "$(COLOR_GREEN)✓ Backup saved to backups/$$BACKUP_NAME$(COLOR_RESET)"

restore: check-ssh ## Restore from backup (usage: make restore BACKUP_FILE=backups/backup-xxx.tar.gz)
	@if [ -z "$(BACKUP_FILE)" ]; then \
		echo "$(COLOR_YELLOW)Please specify BACKUP_FILE=backups/backup-xxx.tar.gz$(COLOR_RESET)"; \
		exit 1; \
	fi
	@echo "$(COLOR_BLUE)Restoring from $(BACKUP_FILE)...$(COLOR_RESET)"
	scp -i $(SSH_KEY) $(BACKUP_FILE) $(PI_USER)@$(PI_IP):/tmp/restore.tar.gz
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo systemctl stop home-automation-rs.service zigbee2mqtt.service mosquitto.service && \
		cd $(DATA_DIR) && \
		sudo tar -xzf /tmp/restore.tar.gz && \
		sudo chown -R $(PI_USER):$(PI_USER) $(DATA_DIR) && \
		rm /tmp/restore.tar.gz && \
		sudo systemctl start mosquitto.service zigbee2mqtt.service home-automation-rs.service"
	@echo "$(COLOR_GREEN)✓ Restore complete$(COLOR_RESET)"

clean: ## Clean local build artifacts
	@echo "$(COLOR_BLUE)Cleaning local build artifacts...$(COLOR_RESET)"
	cargo clean
	rm -rf systemd/ configs/
	@echo "$(COLOR_GREEN)✓ Clean complete$(COLOR_RESET)"

clean-pi: check-ssh ## Remove all deployed files from Raspberry Pi (WARNING: destructive)
	@echo "$(COLOR_YELLOW)WARNING: This will remove all deployed files and data from the Pi$(COLOR_RESET)"
	@echo -n "Are you sure? [y/N] " && read ans && [ $${ans:-N} = y ]
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		sudo systemctl stop home-automation-rs.service zigbee2mqtt.service mosquitto.service && \
		sudo systemctl disable home-automation-rs.service zigbee2mqtt.service && \
		sudo rm -f /etc/systemd/system/home-automation-rs.service && \
		sudo rm -f /etc/systemd/system/zigbee2mqtt.service && \
		sudo systemctl daemon-reload && \
		sudo rm -rf $(DEPLOY_DIR) $(DATA_DIR) $(LOG_DIR)"
	@echo "$(COLOR_GREEN)✓ Raspberry Pi cleaned$(COLOR_RESET)"

pi-shell: check-ssh ## Open SSH shell to Raspberry Pi
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP)

pi-info: check-ssh ## Display Raspberry Pi system information
	@echo "$(COLOR_BLUE)Raspberry Pi System Information:$(COLOR_RESET)"
	@ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		echo '$(COLOR_BOLD)OS Info:$(COLOR_RESET)' && \
		cat /etc/os-release | grep PRETTY_NAME && \
		echo '' && \
		echo '$(COLOR_BOLD)Kernel:$(COLOR_RESET)' && \
		uname -a && \
		echo '' && \
		echo '$(COLOR_BOLD)Memory:$(COLOR_RESET)' && \
		free -h && \
		echo '' && \
		echo '$(COLOR_BOLD)Disk Usage:$(COLOR_RESET)' && \
		df -h / && \
		echo '' && \
		echo '$(COLOR_BOLD)USB Devices:$(COLOR_RESET)' && \
		lsusb | grep -i 'zigbee\|usb' || lsusb"

test-connection: check-ssh ## Test MQTT connection on Raspberry Pi
	@echo "$(COLOR_BLUE)Testing MQTT broker connection...$(COLOR_RESET)"
	ssh -i $(SSH_KEY) $(PI_USER)@$(PI_IP) "\
		timeout 5 mosquitto_sub -h localhost -t test/# -C 1 & \
		sleep 1 && \
		mosquitto_pub -h localhost -t test/hello -m 'Hello from Makefile' && \
		wait"
	@echo "$(COLOR_GREEN)✓ MQTT test complete$(COLOR_RESET)"
