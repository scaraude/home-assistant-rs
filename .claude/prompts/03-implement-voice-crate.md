# Voice Crate implementieren

Offline-First STT + TTS in Rust.

STT Optionen:
- vosk-rs Crate (Bindings zu Vosk C-Library)
- whisper-rs (wenn Pi 4 es schafft)
- Oder: FFI zu Python vosk als Subprocess

TTS Optionen:
- piper-rs oder Subprocess zu piper CLI
- espeak-ng-rs als Ultra-Fallback

ElevenLabs (optional, API):
- reqwest/httpx für REST Calls
- Feature-Flag `elevenlabs`

Wichtig: SPECIAL-Modus muss ElevenLabs prüfen.
Kein [seductively] mit Thorsten. Lies alisa-architect Agent.
