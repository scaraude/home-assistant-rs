# MCP Builder Agent

Baut den MCP Server für Audio Deck. Claude spricht mit dem Haus.

## Pattern (von mcp-loxone, MIT)
- Async Rust + SSE Transport
- PulseEngine MCP Framework (oder eigene Implementation)
- Tools = Aktionen, Resources = Read-only Daten

## Tools (geplant)
```
light_control(room, action, value?)     → Licht steuern
climate_control(room, action, value?)   → Heizung/Klima
device_control(device_id, action)       → Steckdose, Rollladen
announce(text, room?)                   → Ansage über Lautsprecher
music_control(action, query?)           → Musik steuern
scene_activate(scene_name)              → Szene aktivieren
diagnose(section?)                      → System-Diagnose
brain_status()                          → WoL/LM Studio/Grok Status
alisa_mode(mode)                        → Alisa Modus wechseln
```

## Resources (geplant)
```
devices://list                          → Alle Geräte
devices://{id}/state                    → Gerätezustand
sensors://temperature/{room}            → Temperatur
energy://solar                          → FENECON Solar Status
alisa://status                          → Alisa Modus + Capabilities
system://health                         → Diagnose-Ergebnis
```

## SSE Transport
```rust
// axum Route
async fn sse_handler() -> Sse<impl Stream<Item = Event>> {
    let stream = BroadcastStream::new(rx)
        .map(|msg| Event::default().data(msg));
    Sse::new(stream)
}
```

## Claude Desktop Integration
```json
{
    "mcpServers": {
        "audio-deck": {
            "url": "http://PI_IP:8090/sse"
        }
    }
}
```
