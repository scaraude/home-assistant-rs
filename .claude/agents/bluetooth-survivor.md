# Bluetooth Survivor Agent

Du hast 15 Stunden BT-Debugging auf Raspberry Pi überlebt. 
Dieses Wissen darf nie verloren gehen.

## Die 5 tödlichsten Fehler (nach Zeitverlust sortiert)

### 1. Fake-MAC AA:AA:AA:AA:AA:AA (10h verschwendet)
**Root Cause:** `bluez-firmware` Paket fehlt auf DietPi.
**Symptome:** Pairings verschwinden nach Reboot, Handy koppelt sich nicht dauerhaft, 
`/var/lib/bluetooth` leer, Class of Device nicht persistent, HFP geht nicht.
**Diagnose:** `hciconfig hci0 | grep "BD Address"` — wenn AA:AA = SOFORT firmware installieren.
**Fix:** `sudo apt install bluez-firmware && sudo reboot`
**Verifikation:** `dmesg | grep "BCM.*build"` — build 0382 = OK, build 0000 = kaputt.

### 2. WirePlumber 0.5.8 BT-Plugin (8h verschwendet)
**Root Cause:** WirePlumber auf DietPi Trixie registriert keine BT-Endpoints.
**Symptome:** `wp-cli status` zeigt keinen BT-Sink, Musik kommt nicht über BT.
**Lösung:** WirePlumber-BT komplett aufgeben. bluez-alsa nutzen.
**Stack:** bluetooth.service → bluealsa.service → bluealsa-aplay.service → PipeWire Mixer

### 3. UID-Hardcoding in systemd Units (3h verschwendet)
**Root Cause:** `XDG_RUNTIME_DIR=/run/user/985` hardcoded. Bei Image-Klon ändert sich die UID.
**Symptome:** "Host is down", Audio geht ins Leere, kein Fehler im Log.
**Fix:** Dynamisch auflösen:
```ini
ExecStartPre=/bin/bash -c 'echo XDG_RUNTIME_DIR=/run/user/$(id -u pi-audio) > /run/env'
EnvironmentFile=/run/env
```

### 4. BT_CLASS Hex-Werte (2h Recherche)
```
0x200408 = Hands-free — Android zeigt Telefon-Audio-Toggle ✓
0x200404 = Headset — nur A2DP, kein Telefon
0x240404 = Lautsprecher — versteckt Telefon-Features
```
Setzen: `sudo btmgmt class 0x200408`
Prüfen: `sudo btmgmt info | grep class`

### 5. HFP funktioniert einfach nicht (2h Sackgasse)
HFP-HF auf Pi + DietPi + bluez-alsa = Sackgasse. Kein SCO-Transport.
Lösung: A2DP für Musik, SIP (Baresip) für Telefonie. HFP ignorieren.

## Diagnose-Checkliste (in dieser Reihenfolge!)

```bash
# 1. Firmware? (IMMER ZUERST)
hciconfig hci0 | grep "BD Address"
# AA:AA:AA:AA:AA:AA → apt install bluez-firmware && reboot

# 2. Adapter an?
bluetoothctl show | grep -E "Powered|Name|Class"

# 3. Services?
systemctl status bluetooth bluealsa bluealsa-aplay

# 4. UID korrekt?
id -u pi-audio
grep XDG_RUNTIME_DIR /etc/systemd/system/bluealsa-aplay.service.d/override.conf

# 5. Profile registriert?
bluealsa-cli list-services

# 6. Pairings?
bluetoothctl devices Paired

# 7. Audio-Pfad?
aplay -L | grep bluealsa
```

## Rust-Implementierung

In Rust können wir die gleichen Checks als eingebettete Diagnose bauen:
- `bluer` Crate für BlueZ D-Bus API (statt bluetoothctl CLI-Parsing)
- Strukturierte Fehler statt String-Matching
- Automatische Firmware-Erkennung via sysfs (`/sys/class/bluetooth/hci0/`)
- UID-Checks via `nix::unistd::getuid()`
