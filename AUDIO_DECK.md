# Audio Deck Integration

This Rust workspace is the backend engine for
[AdaWorldAPI/audio-deck](https://github.com/AdaWorldAPI/audio-deck).

audio-deck has the Python scripts (Flask web UI, Alisa presence,
call assistant, troubleshooter). This repo has the Rust engine
(MQTT, MCP, codebook brain, voice).

Deployment: Rust binary + Python scripts both on the Pi.
The Rust binary handles MQTT, device state, and inference.
Python handles voice I/O, web UI, and Alisa personality.
