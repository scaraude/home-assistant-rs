# Bluetooth Troubleshooting — Raspberry Pi

Aus 15 Stunden realem Debugging. Jedes Problem hier hat echte Stunden gekostet.

## Diagnose-Reihenfolge (IMMER diese!)

### Schritt 1: Firmware (bevor du irgendetwas anderes tust)
```bash
hciconfig hci0 | grep "BD Address"
```
- `BD Address: AA:AA:AA:AA:AA:AA` → **STOPP. Firmware fehlt.**
  ```bash
  sudo apt install bluez-firmware && sudo reboot
  ```
  Danach nochmal prüfen. Wenn immer noch AA:AA → Hardware-Problem.

- `BD Address: B8:27:EB:xx:xx:xx` oder `88:A2:9E:xx:xx:xx` → OK, weiter.

### Schritt 2: Firmware-Version
```bash
dmesg | grep "BCM.*build"
```
- `build 0000` → Firmware NICHT geladen (trotz bluez-firmware installiert → Reboot vergessen?)
- `build 0382` → OK

### Schritt 3: Adapter Status
```bash
bluetoothctl show
```
Muss zeigen: `Powered: yes`, `Discoverable: yes` (wenn gewünscht), korrekter Name.

### Schritt 4: Services
```bash
systemctl status bluetooth          # BlueZ Daemon
systemctl status bluealsa           # bluez-alsa (A2DP Sink)
systemctl status bluealsa-aplay     # Audio-Brücke zu PipeWire
```

### Schritt 5: Pairings
```bash
bluetoothctl devices Paired
bluetoothctl info XX:XX:XX:XX:XX:XX   # Pro Gerät: Trusted? Connected?
```

## Probleme und Lösungen

### Android zeigt keinen Telefon-Audio-Toggle
**Ursache:** BT_CLASS ist nicht 0x200408.
```bash
sudo btmgmt class 0x200408    # Hands-free Device
# Danach: Pairing auf dem Handy löschen und neu koppeln
```

### Pairings gehen bei Reboot verloren
**Mögliche Ursachen:**
1. Fake-MAC (siehe Schritt 1)
2. `/var/lib/bluetooth` auf tmpfs/overlay
   ```bash
   findmnt -n -o FSTYPE /var/lib/bluetooth
   # tmpfs → Bind-Mount einrichten:
   sudo mkdir -p /mnt/dietpi_userdata/bluetooth
   sudo cp -a /var/lib/bluetooth/* /mnt/dietpi_userdata/bluetooth/
   echo '/mnt/dietpi_userdata/bluetooth /var/lib/bluetooth none bind 0 0' | sudo tee -a /etc/fstab
   ```

### "Host is down" nach Image-Klon
**Ursache:** UID in bluealsa-aplay.service stimmt nicht.
```bash
# Prüfen:
id -u pi-audio
grep XDG_RUNTIME /etc/systemd/system/bluealsa-aplay.service.d/override.conf
# Fixen: UID anpassen, daemon-reload, restart
```

### WirePlumber BT funktioniert nicht
**Das ist bekannt.** WirePlumber 0.5.8 auf DietPi Trixie registriert keine BT-Endpoints.
Lösung: bluez-alsa nutzen, WirePlumber BT ignorieren.

### HFP/Telefon über BT geht nicht
**Das funktioniert nicht zuverlässig auf Pi + DietPi.**
Lösung: SIP über Baresip für Telefonie, BT nur für A2DP (Musik).

## Firmware-Dateien nach Pi-Modell

| Pi | Chip | Firmware |
|---|---|---|
| Pi 3B+ | BCM4345C0 | BCM4345C0.hcd |
| Pi 4 | BCM4345C0 | BCM4345C0.hcd |
| Pi 5 | BCM4375B1 | BCM4375B1.hcd |
| Pi Zero 2 W | BCM4345C0 | BCM4345C0.hcd |
