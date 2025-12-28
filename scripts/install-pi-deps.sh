#!/bin/bash
# Installation script for Raspberry Pi dependencies
# This script installs Mosquitto MQTT broker and Node.js for Zigbee2MQTT

set -e  # Exit on error

echo "=================================="
echo "Installing Home Automation RS Dependencies"
echo "=================================="

# Update system
echo ""
echo "[1/5] Updating system packages..."
sudo apt-get update
sudo apt-get upgrade -y

# Install Mosquitto MQTT broker
echo ""
echo "[2/5] Installing Mosquitto MQTT broker..."
sudo apt-get install -y mosquitto mosquitto-clients

# Enable Mosquitto to start on boot
echo "Enabling Mosquitto service..."
sudo systemctl enable mosquitto
sudo systemctl stop mosquitto  # We'll configure it with our custom config later

# Install Node.js and npm (required for Zigbee2MQTT)
echo ""
echo "[3/5] Installing Node.js and npm..."

# Check if Node.js is already installed
if command -v node &> /dev/null; then
    NODE_VERSION=$(node -v)
    echo "Node.js is already installed: $NODE_VERSION"

    # Check if version is acceptable (v18 or higher recommended for Zigbee2MQTT)
    NODE_MAJOR=$(node -v | cut -d'.' -f1 | sed 's/v//')
    if [ "$NODE_MAJOR" -lt 18 ]; then
        echo "Node.js version is too old. Upgrading..."
        sudo apt-get remove -y nodejs npm || true
    else
        echo "Node.js version is acceptable, skipping installation"
        NODE_INSTALLED=1
    fi
fi

if [ -z "$NODE_INSTALLED" ]; then
    # Install Node.js LTS from NodeSource
    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
    sudo apt-get install -y nodejs
fi

# Verify installations
echo ""
echo "[4/5] Verifying installations..."
echo "Node.js version: $(node -v)"
echo "npm version: $(npm -v)"
echo "Mosquitto version: $(mosquitto -h | head -n1)"

# Install additional useful tools
echo ""
echo "[5/5] Installing additional tools..."
sudo apt-get install -y git curl wget

# Create mosquitto user if it doesn't exist
if ! id -u mosquitto &>/dev/null; then
    echo "Creating mosquitto user..."
    sudo useradd -r -s /bin/false mosquitto
fi

# Install build essentials (might be needed for some npm packages)
echo "Installing build essentials for npm native modules..."
sudo apt-get install -y build-essential python3 python3-dev

echo ""
echo "=================================="
echo "✓ Installation complete!"
echo "=================================="
echo ""
echo "Installed components:"
echo "  - Mosquitto MQTT Broker"
echo "  - Node.js $(node -v)"
echo "  - npm $(npm -v)"
echo ""
echo "Next steps:"
echo "  1. Run 'make deploy-full' from your local machine to complete setup"
echo "  2. Or run individual make targets as needed"
echo ""
