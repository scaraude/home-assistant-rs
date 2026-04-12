# Audio Engineer Agent

PipeWire + ALSA + AEC auf Raspberry Pi. Weiß was funktioniert und was nicht.

## Stack (funktional, produktiv)
```
USB-Mikrofon → ALSA → PipeWire → AEC-Filter → AEC-Source (echo-bereinigt)
                                     ↑
InnoMaker DAC → ALSA → PipeWire → AEC-Sink (Referenz für Echounterdrückung)
                                     ↓
                              Hardware Speaker
```

## Kritische Konfiguration

### AEC (Echo Cancellation)
- Config: `~/.config/pipewire/pipewire.conf.d/20-aec-filter.conf`
- AEC-Source = was das Mikrofon hört MINUS was der Lautsprecher spielt
- ALLES muss durch den AEC-Sink, sonst kein Echo-Cancel
- AUSNAHME: BT-Audio via bluez-alsa bypassed AEC bewusst

### Samplerate
- Lock auf 48kHz in `10-samplerate.conf`
- Ohne Lock: PipeWire wechselt Samplerate je nach Quelle → Knacken

### BT-Audio Bypass
bluez-alsa → PipeWire Mixer (NICHT AEC-Sink)
Das ist korrekt. BT-Audio braucht kein AEC weil es nicht ins Mikro geht.
ABER: Wenn gleichzeitig telefoniert wird UND BT-Musik läuft, hört der
Gesprächspartner die Musik (AEC kann sie nicht subtrahieren).

## ALSA-Geräte finden
```bash
aplay -L | grep ^hw:CARD    # Output (DAC)
arecord -L | grep ^hw:CARD  # Input (Mikrofon)
```
Strings 1:1 in ALSA_OUT_NAME / ALSA_IN_NAME eintragen.

## Häufige Fehler
- "Function not implemented" bei Baresip ring.wav → ansat.so deaktivieren
- PipeWire crashed bei BT-Connect → WirePlumber BT-Plugin deaktiviert lassen
- "Device or resource busy" → Geisterprozess, `fuser /dev/snd/*`
- Kein Sound nach Reboot → `systemctl --user restart pipewire wireplumber`

## Rust-Äquivalente
- `pipewire-rs` Crate für PipeWire-Interaktion
- `alsa-rs` für direkte ALSA-Kontrolle
- `cpal` für plattformübergreifendes Audio (aber weniger Kontrolle)
