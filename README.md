# home-automation-rs

**A full home-automation stack — Rust backend, Svelte dashboard — that runs comfortably on a $15 Raspberry Pi Zero 2 W.**

[![CI](https://github.com/scaraude/home-automation-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/scaraude/home-automation-rs/actions/workflows/ci.yml)
[![Live demo](https://img.shields.io/badge/demo-live-brightgreen)](https://scaraude.github.io/home-automation-rs/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

Zigbee sensors and switches talk to [zigbee2mqtt](https://www.zigbee2mqtt.io/), this service listens on MQTT, stores everything in SQLite, evaluates your automation rules in real time, and serves a modern web dashboard — floor plan, live charts, energy tracking, rule editor. No cloud, no account, no Docker on the device. One ~3 MB binary using 10–20 MB of RAM.

## ✨ Try it now

**[→ Live demo](https://scaraude.github.io/home-automation-rs/)** — the real dashboard running entirely in your browser against a mock backend. No install needed.

## Why this project?

Most self-hosted home-automation platforms assume a Raspberry Pi 4 or better. This project starts from the opposite end: **what can you build when the whole house runs on 416 MB of RAM and an SD card?**

The answer shapes every technical choice:

- **Rust + raw [hyper](https://hyper.rs/)** instead of a full web framework — every KB counts
- **SQLite** with memory-mapped I/O and serialized writes — no database server to feed
- **Svelte** — compiles away, ships almost no runtime to the browser
- **Event-driven, push-only** — MQTT events fan out on a `tokio::broadcast` bus to the DB writer, the automation engine, the in-memory state store, and a WebSocket broadcaster. The UI gets updates in <100 ms with zero polling
- **Smart data retention** — raw readings kept 7 days, then downsampled (deadband + heartbeat for energy, hourly for temperature) so the DB stays small forever
- **Layered caching** for heavy assets like the floor plan SVG (IndexedDB + ETag/304 + gzip)

The result feels fast — on hardware most projects would refuse to boot on.

## Features

- 🗺️ **Floor plan view** — your sensors and switches, live, on your own SVG house plan
- 📈 **Sensor explorer** — temperature, humidity, energy history with automatic time-bucketing
- ⚡ **Energy dashboard** — per-device consumption and cumulative-energy tracking
- 🤖 **Automation rules** — visual editor, AND/OR conditions, compare a sensor to a threshold *or to another sensor*
- 🎛️ **Switch control** — toggle any Zigbee switch from the browser
- 🔌 **Device discovery** — new Zigbee devices are registered automatically, with battery and link-quality monitoring
- 📊 **System monitoring** — CPU, RAM, and per-process metrics of the Pi itself
- 📡 **Real-time everything** — WebSocket event stream, no refresh button

## Architecture

```
        ┌───────────────┐         ┌─────────────┐         ┌─────────────────┐
        │  zigbee2mqtt  │         │  mosquitto  │         │ home-automation │
        │   (systemd)   │◄───────►│   (broker)  │◄───────►│      -rs        │
        └───────┬───────┘  pub/sub└─────────────┘  pub/sub│   (systemd)     │
                │ USB                                     │                 │
        ┌───────▼───────┐                                 │  SQLite + Hyper │
        │ Zigbee dongle │                                 │  HTTP :8082     │
        └───────┬───────┘                                 └────────▲────────┘
                │ Zigbee 3.0                                       │ HTTP + WS
        ┌───────▼───────┐                                  ┌───────▼──────┐
        │Zigbee devices │                                  │    Svelte    │
        │sensors·switch │                                  │  dashboard   │
        └───────────────┘                                  └──────────────┘
```

Inside the backend, every MQTT message becomes a typed `SystemEvent` on a broadcast bus; independent services (DB writer, automation engine, state manager, WebSocket broadcaster) each subscribe to what they need.

## Quick start

**Prerequisites:** Rust 1.83+, Node.js 18+, a Raspberry Pi with systemd, a Zigbee USB dongle. The binary is cross-compiled to static musl (`aarch64-unknown-linux-musl`) with a native toolchain — no Docker; `make setup-cross-compile` installs it (Homebrew on macOS).

```bash
git clone https://github.com/scaraude/home-automation-rs
cd home-automation-rs

# Configure your Pi (host, user, deploy dir)
cp .env.deploy.example .env.deploy && $EDITOR .env.deploy

# Build (cross-compiles to aarch64) and deploy everything:
# binary, frontend, mosquitto, zigbee2mqtt, systemd units
make build
make deploy-full
make start
```

Then open `http://<your-pi>:8082`. 🎉

For local development without a Pi:

```bash
cargo run                              # backend on :8082
cd frontend && npm install && npm run dev   # dashboard with hot reload (mock backend available)
```

## Documentation

| Topic | Where |
|---|---|
| Deployment guide (services, backups, troubleshooting) | [DEPLOYMENT.md](DEPLOYMENT.md) |
| Day-to-day commands cheatsheet | [DEPLOYMENT_CHEATSHEET.md](DEPLOYMENT_CHEATSHEET.md) |
| Performance internals & data retention | [docs/PERFORMANCE_AND_RETENTION.md](docs/PERFORMANCE_AND_RETENTION.md) |
| All Makefile targets | `make help` / [Makefile](Makefile) |

## Project structure

```
src/                  Rust backend
├── mqtt/             MQTT client + zigbee2mqtt message handlers
├── events/           SystemEvent enum — the spine of the app
├── services/         DB writer, automation engine, state manager, WS broadcaster, retention
├── db/               SQLite layer (queries, migrations, retention rollups)
├── http/             Hyper server, API routes, compression
└── automation/       Rule evaluation engine

frontend/             Svelte dashboard (floor plan, sensors, energy, rules, logs)
scripts/              DB backfill, SVG optimization
configs/              systemd units, mosquitto & zigbee2mqtt configs
```

## Contributing

Issues and pull requests are welcome! Before submitting:

```bash
cargo test && cargo fmt --check && cargo clippy   # backend
cd frontend && npm run check                      # frontend
```

Good first areas: new sensor types, time-based automation conditions, notifications. Check the [open issues](https://github.com/scaraude/home-automation-rs/issues).

## License

[MIT](LICENSE)
