# BT Troubleshooting in Rust

Portiere die Python troubleshoot.py BT-Checks nach Rust.

Lies docs/troubleshooting/BLUETOOTH.md für alle bekannten Probleme.

Rust-Ansatz:
- `bluer` Crate für BlueZ D-Bus API
- sysfs für Firmware-Check (/sys/class/bluetooth/hci0/)
- `nix` Crate für UID-Checks
- Strukturierte Fehler (enum statt String-Matching)

Checks zu portieren:
1. Firmware (Fake-MAC Detection)
2. Adapter Status (powered, discoverable, class)
3. Service Status (bluetooth, bluealsa, bluealsa-aplay)
4. UID-Match
5. Pairings + Trust Status
6. Persistence (tmpfs Detection)

Output: JSON (für Web-UI) und farbiges Terminal (für SSH).
