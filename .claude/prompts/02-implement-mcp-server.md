# MCP Server implementieren

Lies .claude/agents/mcp-builder.md für die Tool-Liste.
Referenz: https://github.com/avrabe/mcp-loxone (MIT)

1. axum SSE-Endpoint auf Port 8090
2. Tool-Registry mit JSON-Schema
3. Erstes Tool: `diagnose` — führt Checks aus, gibt JSON zurück
4. Zweites Tool: `announce` — spielt Text über Lautsprecher
5. Claude Desktop Config für lokalen MCP Server testen

Nutze den Event-Bus aus crates/core für interne Kommunikation.
