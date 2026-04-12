# Home Automation Integration

Stefan's System ist noch nicht identifiziert. Baue pluggable Architecture.

1. Trait definieren:
```rust
#[async_trait]
trait HomeDevice {
    async fn discover() -> Vec<Device>;
    async fn control(&self, action: Action) -> Result<()>;
    async fn status(&self) -> DeviceState;
}
```

2. Implementierungen als Feature-Flags:
   - tuya: tinytuya-rs oder MQTT Bridge
   - hue: lighthouse Crate
   - shelly: MQTT native
   - matter: rs-matter

3. Intent-Router in Rust (portiert von Python):
   - Offline: Keyword-Matching (18 Raumnamen, 7 Intent-Kategorien)
   - Online: Grok/LM Studio Klassifikation

4. MQTT als Universalkleber: alle Geräte über Topics
