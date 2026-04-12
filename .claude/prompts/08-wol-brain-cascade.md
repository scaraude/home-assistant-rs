# Wake-on-LAN Brain Cascade in Rust

Portiere integrations/wol_brain.py nach Rust.

Cascade: Pi Codebook → Stefan's PC (WoL) → Railway → Grok API

1. WoL Magic Packet senden (raw socket, 6×FF + 16×MAC)
2. TCP Health-Check (LM Studio Port 1234)
3. OpenAI-kompatible API Client (reqwest)
4. Timeout + Fallback Logic
5. CLI: --status, --wake, --think "text"

Stefan's PC: i7-13800K + RTX 3060 12GB, LM Studio, Gemma 4 27B
WoL: BIOS aktiviert, PC schläft wenn nicht gebraucht
