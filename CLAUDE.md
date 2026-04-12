# Home Automation RS — Claude Code Entry Point

## Was ist das?

Ein Rust-basiertes Smart Home System mit KI-Stimme (Alisa), das autonom auf 
einem Raspberry Pi 4 läuft. Fork von scaraude/home-automation-rs, erweitert 
mit Codebook-Brain (bgz17), MCP Server, und Voice Control.

## Workspace

```
crates/
  core/       57 Dateien  MQTT (rumqttc), SQLite, HTTP, Events, Device State
  mcp-home/   Skeleton    MCP Server — Claude spricht mit dem Haus
  ada-brain/  Skeleton    Codebook Inference (bgz17/PCDVQ, SIMD)
  voice/      Skeleton    Vosk (offline STT) + Piper (offline TTS)
```

## Agents (lies diese zuerst!)

| Agent | Expertise |
|-------|-----------|
| `bluetooth-survivor` | 15h BT-Debugging-Wissen. Fake-MAC, bluez-alsa, UID-Bombe. |
| `audio-engineer` | PipeWire, ALSA, AEC. Was funktioniert, was nicht. |
| `alisa-architect` | Persona, Sicherheit (Thomas-Proof, Thorsten-Proof). |
| `rust-pi-specialist` | Cross-Compile, NEON SIMD, Binary-Size, RAM. |
| `mcp-builder` | MCP Server Pattern, SSE, Tools/Resources. |

## Prompts (Entwicklungspfade)

```
/prompt 01-get-started              Erstes Build + Cross-Compile
/prompt 02-implement-mcp-server     MCP für Claude Desktop
/prompt 03-implement-voice-crate    Vosk + Piper in Rust
/prompt 04-implement-codebook-brain bgz17 Inference auf Pi 4
/prompt 05-add-mqtt-devices         Zigbee, Tuya, Shelly
/prompt 06-bt-troubleshoot-in-rust  Python Checks → Rust portieren
/prompt 07-home-automation          Pluggable Device Trait
/prompt 08-wol-brain-cascade        Wake-on-LAN + LM Studio
```

## Quick Start

```bash
# Build (x86):
cargo build --release

# Cross-Compile (Pi 4):
cargo build --release --target aarch64-unknown-linux-gnu

# Einzelnen Crate:
cargo build -p home-core --release
cargo build -p mcp-home --release
```

## Verwandte Repos

| Repo | Was |
|------|-----|
| AdaWorldAPI/audio-deck | Python: Web-UI, Alisa Persona, Voice Messages, Docs |
| AdaWorldAPI/lance-graph | Rust: Graph DB, DataFusion, Cypher |
| AdaWorldAPI/ndarray | Rust: bgz17 Kompression, SIMD Kernels |

## Architektur-Prinzipien

1. **Offline-First**: Alles funktioniert ohne Internet
2. **Codebook statt Matmul**: O(1) Lookup, kein GPU nötig
3. **MQTT als Kleber**: Alle Smart-Home-Geräte über Topics
4. **Matryoshka**: Jeder Crate kann die Schale werden
5. **Thomas-Proof**: SPECIAL pausiert bei Anruf, Kontext gelöscht
6. **Thorsten-Proof**: SPECIAL blockt bei Piper-Fallback
7. **3MB Binary**: opt-level z, LTO, strip, panic=abort

## Troubleshooting

Bevor du irgendetwas debuggst, lies:
- `docs/troubleshooting/BLUETOOTH.md` — die 5 tödlichsten BT-Fehler
- `docs/troubleshooting/AUDIO.md` — PipeWire + AEC
- `docs/troubleshooting/NETWORK_SIP.md` — Baresip + Snapcast

## SIMD Tiers (ada-brain)

| Feature | Hardware | tok/s |
|---------|----------|-------|
| `amx` | Sapphire Rapids | 380.000 |
| `avx512` | Xeon / i9 | 10.000-50.000 |
| `avx2` | Consumer i5/i7 | 3.000-10.000 |
| `neon` | Pi 4 Cortex-A72 | 500-5.000 |
| `scalar` | Alles | 50-500 |
