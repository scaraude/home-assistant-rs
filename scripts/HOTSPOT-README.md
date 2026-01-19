# Pi Zero 2W Isolated Hotspot Setup

Simple, lazy setup for a 24/7 isolated Wi-Fi hotspot that runs alongside your existing Wi-Fi connection using a virtual interface.

## What It Does

- Creates a permanent Wi-Fi hotspot on your Pi Zero 2W
- Allows max 2 devices to connect simultaneously
- **Isolated network**: no internet access, only access to your web app at `:8082`
- Your Pi stays connected to your home Wi-Fi (for normal operation)
- Your Pi remains visible on your home network
- Zero manual configuration after initial setup

## Quick Setup

```bash
make setup-hotspot
```

That's it. The script will prompt you to confirm, install dependencies, configure everything, and ask if you want to reboot.

## Default Settings

- **SSID**: `PiHomeAuto`
- **Password**: `automation2026`
- **Pi IP on hotspot**: `192.168.50.1`
- **Max clients**: 2

### Customize (Optional)

Before running, you can set environment variables:

```bash
HOTSPOT_SSID="MyCustomName" HOTSPOT_PASSWORD="MyPassword123" make setup-hotspot
```

Or edit `scripts/setup-hotspot.sh` directly (lines 16-18).

## Access Your Web App

Once connected to the hotspot:

```
http://192.168.50.1:8082
```

## How It Works

1. **iw** creates a virtual interface `uap0` from the physical `wlan0`
2. **hostapd** creates the access point on `uap0`
3. **dnsmasq** provides DHCP to assign IPs (192.168.50.2-3)
4. **iptables** blocks all forwarding from `uap0` (ensures isolation)
5. **wlan0** remains free for NetworkManager to connect to your home Wi-Fi
6. All services start automatically on boot via systemd

## Network Isolation

The hotspot is completely isolated:
- No DNS resolution
- No internet access
- No access to your home network
- Clients can only reach the Pi itself

This is enforced by:
- Virtual interface `uap0` (separate from `wlan0`)
- `ip_forward=0` (no routing)
- `iptables -A FORWARD -i uap0 -j REJECT` (blocks all forwarding)
- dnsmasq configured with `no-resolv` (no DNS)

## Troubleshooting

### Check hotspot status
```bash
ssh ludovic@Gholam.local
sudo systemctl status hostapd
sudo systemctl status dnsmasq
sudo systemctl status hotspot-setup
```

### View logs
```bash
sudo journalctl -u hostapd -f
sudo journalctl -u dnsmasq -f
```

### Check IP configuration
```bash
ip addr show uap0
# Should show: 192.168.50.1/24

ip addr show wlan0
# Should show your home Wi-Fi IP (e.g., 10.x.x.x or 192.168.1.x)
```

### Manually restart hotspot
```bash
sudo systemctl restart hotspot-setup
sudo systemctl restart hostapd
sudo systemctl restart dnsmasq
```

### Remove hotspot completely
```bash
ssh ludovic@Gholam.local
sudo systemctl stop hostapd dnsmasq hotspot-setup
sudo systemctl disable hostapd dnsmasq hotspot-setup
sudo rm /etc/systemd/system/hotspot-setup.service
sudo rm /usr/local/bin/hotspot-setup.sh
sudo apt remove --purge hostapd dnsmasq
```

## Architecture

```
┌────────────────────────────────────────────────┐
│  Pi Zero 2W                                    │
│                                                │
│  Physical: wlan0                               │
│    ├─→ Home Wi-Fi (NetworkManager)             │
│    │   10.x.x.x or 192.168.1.x                 │
│    │                                           │
│    └─→ Virtual: uap0 (hotspot)                 │
│        192.168.50.1                            │
│        │                                       │
│        └─→ Hotspot Clients (max 2)             │
│            192.168.50.2-3                      │
│            │                                   │
│            └─→ Can only access:                │
│                http://192.168.50.1:8082        │
│                                                │
│                ✗ No internet                   │
│                ✗ No home network               │
└────────────────────────────────────────────────┘
```

## Files Created

- `/etc/hostapd/hostapd.conf` - Access point config (uses `uap0`)
- `/etc/dnsmasq.conf` - DHCP server config
- `/etc/systemd/system/hotspot-setup.service` - Systemd service
- `/usr/local/bin/hotspot-setup.sh` - Virtual interface setup script
- `/etc/NetworkManager/conf.d/unmanaged-uap0.conf` - Prevents NetworkManager from managing `uap0`

## Notes

- The hotspot runs 24/7, not as a fallback
- Uses a **virtual interface** (`uap0`) created from the physical `wlan0`
- Original configs are backed up with timestamps
- NetworkManager handles the home Wi-Fi connection on `wlan0` automatically
- The Pi Zero 2W can handle both connections simultaneously on one physical adapter
- No USB Wi-Fi dongle required!
