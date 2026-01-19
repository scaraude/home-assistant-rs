#!/bin/bash
# Setup isolated Wi-Fi hotspot on Pi Zero 2W using virtual interface
# The Pi will:
# - Stay connected to your home Wi-Fi on wlan0 (for internet/network access)
# - Run a separate hotspot 24/7 on uap0 (no internet, just access to :8082)
# - Support max 2 concurrent connections on hotspot

set -e

echo "=== Pi Zero 2W Isolated Hotspot Setup (Virtual Interface) ==="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root: sudo $0"
  exit 1
fi

# Variables (customize these)
HOTSPOT_SSID="${HOTSPOT_SSID:-Gholam}"
HOTSPOT_PASSWORD="${HOTSPOT_PASSWORD:-Gholam619496}"
HOTSPOT_IP="192.168.50.1"
DHCP_RANGE_START="192.168.50.2"
DHCP_RANGE_END="192.168.50.3"  # Max 2 clients

echo "Hotspot SSID: $HOTSPOT_SSID"
echo "Hotspot Password: $HOTSPOT_PASSWORD"
echo "Hotspot IP: $HOTSPOT_IP"
echo "Max clients: 2"
echo ""
echo "Strategy: Creating virtual interface 'uap0' for hotspot"
echo "  - wlan0: Your home Wi-Fi connection"
echo "  - uap0: Isolated hotspot (virtual interface)"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  exit 1
fi

# Install required packages
echo "Installing required packages..."
apt update
apt install -y hostapd dnsmasq iptables-persistent iw

# Stop services during configuration
systemctl stop hostapd dnsmasq 2>/dev/null || true

# Backup existing configs
timestamp=$(date +%Y%m%d_%H%M%S)
[ -f /etc/hostapd/hostapd.conf ] && cp /etc/hostapd/hostapd.conf /etc/hostapd/hostapd.conf.bak.$timestamp
[ -f /etc/dnsmasq.conf ] && cp /etc/dnsmasq.conf /etc/dnsmasq.conf.bak.$timestamp

# Configure hostapd to use virtual interface
cat > /etc/hostapd/hostapd.conf << EOF
# Use virtual interface for AP
interface=uap0
driver=nl80211

# Network configuration
ssid=$HOTSPOT_SSID
hw_mode=g
channel=6
ieee80211n=1
wmm_enabled=1

# Security configuration
auth_algs=1
wpa=2
wpa_passphrase=$HOTSPOT_PASSWORD
wpa_key_mgmt=WPA-PSK
rsn_pairwise=CCMP

# Limit to 2 clients
max_num_sta=2
EOF

# Configure dnsmasq (DHCP server)
cat > /etc/dnsmasq.conf << EOF
# Listen only on uap0 (virtual interface)
interface=uap0

# Bind to the interface to make sure we aren't sending DHCP requests elsewhere
bind-interfaces

# DHCP range - max 2 clients
dhcp-range=$DHCP_RANGE_START,$DHCP_RANGE_END,255.255.255.0,12h

# No DNS - this is an isolated network
no-resolv

# Don't forward DNS requests (isolated network)
EOF

# Create systemd service to setup virtual interface
cat > /etc/systemd/system/hotspot-setup.service << 'EOF'
[Unit]
Description=Setup isolated hotspot network (virtual interface)
After=network.target
Before=hostapd.service dnsmasq.service

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/local/bin/hotspot-setup.sh

[Install]
WantedBy=multi-user.target
EOF

# Create the setup script for virtual interface
cat > /usr/local/bin/hotspot-setup.sh << 'EOF'
#!/bin/bash
# Create virtual interface and configure it for hotspot

set -e

# Wait for wlan0 to be available
timeout=30
while [ $timeout -gt 0 ] && ! ip link show wlan0 &>/dev/null; do
  sleep 1
  ((timeout--))
done

if ! ip link show wlan0 &>/dev/null; then
  echo "wlan0 not available"
  exit 1
fi

# Remove existing virtual interface if it exists
iw dev uap0 del 2>/dev/null || true

# Create virtual interface uap0 from wlan0
echo "Creating virtual interface uap0..."
iw dev wlan0 interface add uap0 type __ap

# Wait a moment for interface to be created
sleep 1

# Configure static IP on uap0
echo "Configuring IP on uap0..."
ip addr flush dev uap0 2>/dev/null || true
ip addr add 192.168.50.1/24 dev uap0
ip link set uap0 up

# No forwarding - this is an isolated network
echo 0 > /proc/sys/net/ipv4/ip_forward

# Block all forwarding from uap0 to ensure isolation
iptables -D FORWARD -i uap0 -j REJECT 2>/dev/null || true
iptables -A FORWARD -i uap0 -j REJECT

# Save iptables rules
netfilter-persistent save 2>/dev/null || true

echo "Virtual interface uap0 ready"
EOF

chmod +x /usr/local/bin/hotspot-setup.sh

# Prevent NetworkManager from managing uap0
if [ -d /etc/NetworkManager ]; then
  mkdir -p /etc/NetworkManager/conf.d
  cat > /etc/NetworkManager/conf.d/unmanaged-uap0.conf << EOF
[keyfile]
unmanaged-devices=interface-name:uap0
EOF

  # Restart NetworkManager if it's running
  if systemctl is-active --quiet NetworkManager; then
    systemctl restart NetworkManager
  fi
fi

# Enable and start services
systemctl unmask hostapd
systemctl enable hotspot-setup
systemctl enable hostapd
systemctl enable dnsmasq

echo ""
echo "=== Configuration Complete ==="
echo ""
echo "Your Pi will now run a Wi-Fi hotspot with these settings:"
echo "  SSID: $HOTSPOT_SSID"
echo "  Password: $HOTSPOT_PASSWORD"
echo "  Pi IP: $HOTSPOT_IP"
echo "  Max clients: 2"
echo ""
echo "Network configuration:"
echo "  wlan0: Connected to your home Wi-Fi (managed by NetworkManager)"
echo "  uap0: Hotspot interface (isolated, no internet)"
echo ""
echo "To access your web app from a connected device:"
echo "  http://$HOTSPOT_IP:8082"
echo ""
echo "The hotspot network is ISOLATED (no internet access)."
echo "Your Pi will remain connected to your home Wi-Fi on wlan0."
echo ""
echo "Services will start on next reboot."
echo ""
read -p "Reboot now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo "Rebooting..."
  reboot
else
  echo "Please reboot manually to apply changes: sudo reboot"
fi
