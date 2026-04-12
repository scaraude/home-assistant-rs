# Rust on Pi 4 Specialist

Cross-Compile, SIMD, ARM64-Besonderheiten.

## Cross-Compile Setup
```bash
# Einmalig auf der Workstation:
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu

# In .cargo/config.toml:
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

# Build:
cargo build --release --target aarch64-unknown-linux-gnu
# → target/aarch64-unknown-linux-gnu/release/home-automation-rs (~3MB)
```

## SIMD auf ARM64
- Cortex-A72 hat 128-bit NEON (2 Pipelines)
- KEIN AVX-512, kein AMX
- `std::arch::aarch64::*` für NEON Intrinsics
- `#[cfg(target_arch = "aarch64")]` für bedingte Kompilation
- Feature-Flag: `--features neon`

## Codebook Inference Performance
```
Matmul (traditionell):   y = W·x        → GPU nötig, Pi zu langsam
Codebook (bgz17):        y = CB[idx[x]]  → Lookup, RAM-bound, Pi reicht

NEON Kernel: vld1q_f32 + vaddq_f32, 4 floats parallel
Projektion: 500-5000 tok/s auf Pi 4 (vs. 380K auf AMX)
Für 50 Tokens (Alisa-Antwort): 10-100ms
```

## Binary-Größe optimieren
```toml
[profile.release]
opt-level = "z"      # Size statt Speed
lto = true           # Link-Time Optimization
codegen-units = 1    # Bessere Optimierung
strip = true         # Debug-Symbole weg
panic = "abort"      # Kein Unwind-Code
```
Ergebnis: ~3MB Binary statt ~30MB.

## Wichtige Crates für Pi
- `rumqttc`: MQTT (async, klein)
- `rusqlite`: SQLite mit bundled (kein System-libsqlite nötig)
- `axum`: HTTP Server (leichter als actix)
- `bluer`: BlueZ D-Bus API
- `tokio`: Async Runtime (multi-thread oder current-thread für weniger RAM)

## RAM-Bewusstsein
Pi 4 hat 2/4/8GB. Tokio current_thread statt multi_thread spart ~50MB.
```rust
#[tokio::main(flavor = "current_thread")]
async fn main() { ... }
```
