# Netzwerk & SIP Troubleshooting

## Baresip

### Status 98: Address already in use
Geisterprozess oder System-Service Konflikt.
```bash
systemctl --user stop baresip
sudo systemctl stop baresip 2>/dev/null    # System-Service masked?
sleep 2
systemctl --user start baresip
```

### UA not found
Mehrfach-Registrierung an der Fritz!Box (bindings > 1).
In Fritz!Box: Telefoniegerät löschen und neu anlegen.

### ctrl_tcp nicht erreichbar
```bash
# Prüfen ob Port 4444 offen:
nc -z 127.0.0.1 4444 && echo OK || echo CLOSED
# Falls closed: ctrl_tcp.so in ~/.baresip/config aktiviert?
```

### ansat.so Fehler
`ring.wav` wirft "Function not implemented" und blockiert Abheben.
Lösung: `ansat.so` in Baresip-Config deaktivieren.

## Snapcast

```bash
systemctl status snapclient
nc -z SNAPCAST_SERVER 1704 && echo OK || echo CLOSED
```

## Kontakte

```bash
# DB vorhanden?
ls -la /var/lib/pi-audio/contacts.db
# Einträge?
sqlite3 /var/lib/pi-audio/contacts.db "SELECT COUNT(*) FROM contacts"
# Sync manuell:
systemctl start audio-contacts-sync.service
```
