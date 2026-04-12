# Audio Troubleshooting — PipeWire + ALSA

## Diagnose

```bash
# ALSA-Geräte
aplay -L | grep ^hw:CARD     # Output
arecord -L | grep ^hw:CARD   # Input

# PipeWire
systemctl --user status pipewire wireplumber

# AEC-Node
pw-cli ls Node | grep -i echo

# Samplerate
pw-cli dump | grep "rate ="
```

## Probleme

### Kein Sound
1. `aplay -L` → kein hw:CARD → dtoverlay in /boot/config.txt prüfen
2. PipeWire down → `systemctl --user restart pipewire`
3. Falsche ALSA_OUT_NAME → `aplay -L | grep ^hw:CARD` und Config anpassen

### Echo bei Telefonaten
AEC funktioniert nicht. Prüfen:
1. Alle Audio-Quellen durch AEC-Sink? (BT bypassed absichtlich)
2. AEC-Config vorhanden? `~/.config/pipewire/pipewire.conf.d/20-aec-filter.conf`
3. Mikrofon und Speaker auf gleicher Samplerate? (48kHz Lock)

### Knacken/Klicken
Samplerate-Wechsel. Fix: `10-samplerate.conf` mit 48kHz Lock.

### "Device or resource busy"
```bash
fuser /dev/snd/*          # Wer hält das Device?
kill <PID>                # Geisterprozess
systemctl --user restart pipewire
```
