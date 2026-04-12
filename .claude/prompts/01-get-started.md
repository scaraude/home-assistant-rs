# Get Started — Erstes Build

1. Lies CLAUDE.md für den Projekt-Überblick
2. Lies docs/troubleshooting/ für Kontext aus 15h Debugging
3. `cargo build --release` — kompiliert der core-Crate?
4. Fix eventuelle Dependency-Probleme
5. `cargo build --release --target aarch64-unknown-linux-gnu` — Cross-Compile
6. Wie groß ist das Binary? Ziel: <5MB

Der core-Crate (crates/core) ist der scaraude Fork. 
Er sollte kompilieren. Die anderen Crates (mcp-home, ada-brain, voice) 
sind Skeletons mit TODOs.
