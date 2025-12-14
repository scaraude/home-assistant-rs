# Detailed Implementation Plan

This plan covers the architectural refactor to move `home-assistant-rs` toward an event-driven core with WebSocket-powered real-time updates. Each session is intentionally scoped so progress can be checkpointed before moving on.

---

For AI: add marks when you finish to dev a session

## Session 1 · Event Bus Foundation (2–3 hours) ✅ Completed

- **Goal**: Introduce strongly typed events and a broadcast bus.
- **Files**: `src/events/mod.rs`, `src/events/bus.rs`.

### Step 1.1 · Define event types (`src/events/mod.rs`)

- Create a `SystemEvent` enum with variants:
  - `SensorReading { device_id, reading, timestamp }`
  - `SwitchState { device_id, state, timestamp }`
  - `DeviceState { device_id, battery, link_quality, timestamp }`
  - `DeviceDiscovered { device_id, mqtt_topic, capability, timestamp }`
  - `AutomationTriggered { rule_id, actions, timestamp }`
  - `AutomationExecuted { rule_id, success, error, timestamp }`
- Implement helpers:
  - `event_type() -> &'static str`
  - `timestamp() -> DateTime<Utc>`
  - `device_id() -> Option<&str>`
- Derive `Debug`, `Clone`, `Serialize`, `Deserialize`.

### Step 1.2 · Add the event bus wrapper (`src/events/bus.rs`)

- Back the bus with `tokio::sync::broadcast::channel(1000)`.
- Wrapper struct: `EventBus { sender: broadcast::Sender<SystemEvent> }`.
- Methods:
  - `new(capacity: usize) -> Self`
  - `publish(&self, event: SystemEvent) -> Result<usize, broadcast::error::SendError<SystemEvent>>`
  - `subscribe(&self) -> broadcast::Receiver<SystemEvent>`
  - `receiver_count(&self) -> usize`
- Implement `Clone` for `EventBus`.

### Step 1.3 · Unit tests

- Verify event serialization/deserialization.
- Test publish/subscribe flow.
- Ensure lagged receiver errors are surfaced.

**Checkpoint 1**: ✅ Events serialize; ✅ Event bus tests pass; ✅ No regressions.

---

## Session 2 · State Manager Service (1–2 hours) ✅ Completed

- **Goal**: Consume events and keep in-memory state authoritative.
- **Files**: `src/services/mod.rs`, `src/services/state_manager.rs`.

### Step 2.1 · Implement the service (`state_manager.rs`)

- Struct: `StateManagerService { device_state, switch_state, event_rx }`.
- `run(mut self)`:
  - Loop over `event_rx.recv().await`.
  - Handle `DeviceState` → update `DeviceStateStore`.
  - Handle `SwitchState` → update `SwitchStateStore`.
  - On `RecvError::Lagged` log a warning; on `RecvError::Closed` exit loop.

### Step 2.2 · Logging

- Log service start.
- Log state mutations at debug level.
- Log lagged receivers at warn level.

**Checkpoint 2**: ✅ Service compiles; ✅ Works under `tokio::spawn`; ✅ Still isolated.

---

## Session 3 · DB Writer Service (2 hours) ✅ Completed

- **Goal**: Persist events to SQLite.
- **Files**: `src/services/db_writer.rs`, `src/db/schema.rs`, `src/db/queries/mod.rs`, `src/db/queries/switch.rs`.

### Step 3.1 · Schema updates (`src/db/schema.rs`)

```sql
CREATE TABLE IF NOT EXISTS switch_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    state INTEGER NOT NULL, -- 0 = off, 1 = on
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);

CREATE INDEX idx_switch_state_latest
    ON switch_state(device_id, timestamp DESC);
```

### Step 3.2 · Switch queries (`src/db/queries/switch.rs`)

- `insert_switch_state(device_id, state, timestamp)`.
- `get_latest_switch_state(device_id) -> Option<bool>`.
- Export module from `src/db/queries/mod.rs`.

### Step 3.3 · Writer service (`src/services/db_writer.rs`)

- Struct: `DbWriterService { db, event_rx }`.
- `run(mut self)` handles:
  - `SensorReading` → `db.insert_reading()`.
  - `DeviceState` → update battery, link quality, last seen.
  - `SwitchState` → `db.insert_switch_state()`.
  - `AutomationExecuted` → `db.insert_execution_log()`.
  - Lagged receivers → warn.
- Track event count and log stats every 100 events.

**Checkpoint 3**: ✅ Schema migrates locally; ✅ Queries tested; ✅ Service compiles.

---

## Session 4 · MQTT Handler Refactor (2–3 hours)

- **Goal**: Replace dual writes with event publication.
- **Files**: `src/mqtt/handlers/device.rs`, `src/mqtt/event_loop.rs`, `src/mqtt/client.rs`.

### Step 4.1 · Update `MqttClient`

- Change constructor signature from returning `(Self, mpsc::Receiver<SensorReading>)` to `Self` plus `EventBus`.
- Remove `mpsc::channel` creation; store `EventBus` inside the client.

### Step 4.2 · Update event loop

- `spawn_event_loop(..., event_bus: EventBus)`.
- Pass the bus into `handle_device_message`.

### Step 4.3 · Publish events in handlers

- Replace device state and DB dual writes with `SystemEvent::DeviceState`.
- Replace switch cache writes with `SystemEvent::SwitchState`.
- Replace raw channel sends with `SystemEvent::SensorReading`.

**Checkpoint 4**: ✅ MQTT handler emits events only; ✅ `mpsc` removed; ✅ Builds.

---

## Session 5 · Automation Service (2–3 hours)

- **Goal**: Run automation logic as an event consumer.
- **Files**: `src/services/automation.rs`, `src/automation/mod.rs`.

### Step 5.1 · New automation service

- Struct fields: `db`, `mqtt_client`, `device_state`, `switch_state`, `event_bus`, `event_rx`, `last_triggered`, `debounce_seconds`.
- `run(mut self)`:
  - Receive events, process only `SensorReading`.
  - Call `evaluate_reading_internal(reading)`.
  - Publish `AutomationTriggered`.
  - Execute actions via `mqtt_client.publish_command()`.
  - Publish `AutomationExecuted` with result.
- `evaluate_reading_internal()` mirrors existing logic.

### Step 5.2 · Retain HTTP API surface (`automation/mod.rs`)

- Keep `AutomationEngine` for REST calls (CRUD on rules).
- Remove direct evaluation logic; delegate to service.

**Checkpoint 5**: ✅ Service compiles; ✅ Logic extracted; ✅ Automation events emitted.

---

## Session 6 · Main Integration (1–2 hours)

- **Goal**: Wire services and remove legacy plumbing.
- **File**: `src/main.rs`.

### Step 6.1 · Initialize the bus

```rust
let event_bus = EventBus::new(1000);
info!("Event bus initialized with capacity 1000");
```

### Step 6.2 · Update MQTT client creation

```rust
let mqtt_client = MqttClient::new(
    &mqtt_broker,
    mqtt_port,
    "home-assistant-rs",
    event_bus.clone(),
);
```

### Step 6.3 · Remove legacy DB writer task

- Delete the `tokio::spawn` block that consumed `readings_rx`.

### Step 6.4 · Spawn new services

- `DbWriterService::new(db.clone(), event_bus.subscribe())`.
- `StateManagerService::new(device_state.clone(), switch_state.clone(), event_bus.subscribe())`.
- `AutomationService::new(db.clone(), mqtt_client.clone(), device_state.clone(), switch_state.clone(), event_bus.clone(), event_bus.subscribe())`.

### Step 6.5 · Remove HTTP cache init

- Drop `ResponseCache` creation and cleanup task.

### Step 6.6 · Update HTTP server construction

```rust
let server = HttpServer::new(db, mqtt_client, switch_state, device_state, http_addr);
```

**Checkpoint 6**: ✅ `main.rs` builds; ✅ Services spawned; ✅ Old channel removed.

---

## Session 7 · HTTP Server Cleanup (1 hour)

- **Goal**: Remove backend cache usage.
- **Files**: `src/http/mod.rs`, `src/http/routes.rs`.

### Step 7.1 · Update server struct

- Remove `cache: Arc<ResponseCache>`.
- Adjust `new()` and `run()` signatures/fields.

### Step 7.2 · Drop cache lookups

- Delete cached-path logic and `is_cacheable_path()`.

### Step 7.3 · Update routes

- Remove cache parameters from `serve_sensors`, `serve_readings`, `serve_log_view`.
- Always hit the database directly.

**Checkpoint 7**: ✅ HTTP server builds; ✅ Manual route test passes.

---

## Session 8 · WebSocket Infrastructure (2–3 hours)

- **Goal**: Serve events over WebSocket.
- **Files**: `Cargo.toml`, `src/services/websocket.rs`, `src/http/websocket.rs`.

### Step 8.1 · Dependencies (`Cargo.toml`)

```toml
tokio-tungstenite = "0.24"
futures-util = "0.3"
```

### Step 8.2 · Broadcaster service

- Struct: `WebSocketBroadcaster { event_rx, clients: Arc<RwLock<HashMap<ClientId, UnboundedSender<String>>>>, next_client_id }`.
- `run(mut self)` reads from the bus, serializes to JSON, and broadcasts to registered clients (drops disconnected ones).
- `add_client()` registers a client and returns `(ClientId, UnboundedReceiver<String>)`.
- `remove_client(id)` cleans up.

### Step 8.3 · HTTP upgrade handler

- `handle_websocket_upgrade(req, broadcaster)`:
  - Validate upgrade headers.
  - Spawn upgrade via `hyper::upgrade::on(req)`.
  - Respond `101 Switching Protocols`.
- `handle_websocket_connection(upgraded, broadcaster)`:
  - Convert to `WebSocketStream`.
  - Split sender/receiver.
  - Register client with broadcaster; forward outbound events.
  - Handle ping/pong/close.

**Checkpoint 8**: ✅ New deps compile; ✅ Broadcaster + endpoint compile.

---

## Session 9 · WebSocket Integration (1 hour)

- **Goal**: Expose `/ws` and run broadcaster.
- **Files**: `src/main.rs`, `src/http/mod.rs`.

### Step 9.1 · Spawn broadcaster

```rust
let ws_broadcaster = Arc::new(WebSocketBroadcaster::new(event_bus.subscribe()));
let ws_broadcaster_clone = ws_broadcaster.clone();
tokio::spawn(async move { ws_broadcaster_clone.run().await });
```

### Step 9.2 · HTTP route

- Inject `ws_broadcaster` into `HttpServer`.
- In `handle_request`, route `("GET", "/ws")` to `websocket::handle_websocket_upgrade`.

**Checkpoint 9**: ✅ `/ws` accessible; ✅ Upgrade confirmed with `wscat`.

---

## Session 10 · Frontend WebSocket Client (2 hours)

- **Goal**: Replace polling with push updates.
- **Files**: `frontend/src/lib/websocket.ts`, `frontend/src/App.svelte`, `frontend/src/lib/stores/dataCache.ts`.

### Step 10.1 · Client helper (`websocket.ts`)

- Implement `EventStreamClient`:
  - `connect()` establishes `WebSocket`.
  - `onmessage` parses JSON and emits events.
  - `onerror/onclose` trigger exponential backoff reconnects.
  - `disconnect()` cleans up listeners and sockets.
- Export singleton `eventStream`.

### Step 10.2 · Update `App.svelte`

- `onMount`:
  - Fetch initial data via REST.
  - `eventStream.connect()`.
  - Subscribe to `eventStream.events` to update local state.
- `onDestroy`: `eventStream.disconnect()`.
- Remove `setInterval` polling.

### Step 10.3 · Update data cache store

- Add helpers to merge incoming events:
  - `mergeSensorReading`.
  - `updateSwitchState`.
  - `updateDeviceState`.

**Checkpoint 10**: ✅ Frontend receives live updates; ✅ Auto-reconnect ok; ✅ No polling.

---

## Session 11 · Testing & Validation (2–3 hours)

- **Goal**: Validate end-to-end before deployment.

### Step 11.1 · Unit tests

- Run `cargo test`.
- Confirm event serialization and DB switch queries.

### Step 11.2 · Integration tests

- Run `cargo run` and ensure services start cleanly.
- Simulate MQTT traffic (`mosquitto_pub` or helper script).
- Verify DB inserts for readings + switch states.
- Load frontend, ensure real-time updates and automations.
- Restart backend to verify WebSocket reconnection.

### Step 11.3 · Performance checks

- `ps aux | grep home-assistant` → ~16 MB.
- `ls -lh target/release/home-assistant-rs` → ~3.2 MB binary.
- Logs should show no lag warnings; monitor RSS over 1 hour.

### Step 11.4 · Bug fixes

- Document issues, fix critical ones, defer minor bugs post-deployment.

**Checkpoint 11**: ✅ Tests pass; ✅ Real-time path stable; ✅ Performance acceptable.

---

## Session 12 · Pi Deployment & Monitoring (1–2 hours)

- **Goal**: Deploy to Pi Zero and validate in situ.

### Step 12.1 · Build for ARM

```sh
make build
```

### Step 12.2 · Deploy artifacts

```sh
make quick-deploy
make quick-deploy-frontend
```

### Step 12.3 · Monitor services

```sh
make logs
make status
ssh ludovic@Gholam.local "free -h"
ssh ludovic@Gholam.local "ps aux | grep home-assistant"
ssh ludovic@Gholam.local "ss -tn | grep :8080"
```

### Step 12.4 · Functional validation

- Dashboard loads, sensor readings live-update, automations trigger, switch control works, WebSocket reconnect OK, no lag warnings, memory ~16 MB.

### Step 12.5 · 24-hour soak

- Monitor memory, lag, crashes. Expect stability.

**Checkpoint 12**: ✅ Pi deployment stable; ✅ Real-time path verified; ✅ No perf issues.

---

## Session 13 · Cleanup & Documentation (1 hour)

- **Goal**: Remove dead code and update docs.

### Step 13.1 · Delete unused cache code

```sh
rm src/cache/mod.rs
```

### Step 13.2 · Update `CLAUDE.md`

- Document new architecture, data flow, event bus, WebSocket layer, and performance metrics. Remove cache references.

### Step 13.3 · Update `README.md`

- Mention event-driven design, `/ws` endpoint, and revised system requirements.

### Step 13.4 · Commit

```sh
git add .
git commit -m "Refactor to event-driven architecture with WebSocket

- Replace polling with WebSocket real-time updates
- Event bus (tokio::broadcast) for SPMC event distribution
- 4 independent services: DB Writer, Automation, WebSocket Broadcaster, State Manager
- Remove backend cache (500KB-1MB memory savings)
- Persist switch state to database
- 150x faster frontend updates, 96% bandwidth reduction
- +1MB RAM, +200KB binary (acceptable for Pi Zero 2W)

Closes #<issue_number>"
```

**Checkpoint 13**: ✅ Dead code removed; ✅ Docs updated; ✅ Changes committed.

---

## Session Summary

| #   | Session                  | Duration | Checkpoint                          | Resume? |
| --- | ------------------------ | -------- | ----------------------------------- | ------- |
| 1   | Event Bus Foundation     | 2–3h     | Events + bus compile, tests pass ✅ | Yes     |
| 2   | State Manager Service    | 1–2h     | Service compiles ✅                 | Yes     |
| 3   | DB Writer Service        | 2h       | Service + DB schema compile ✅      | Yes     |
| 4   | MQTT Handler Refactor    | 2–3h     | MQTT publishes events ✅            | Yes     |
| 5   | Automation Service       | 2–3h     | Service compiles ✅                 | Yes     |
| 6   | Integration in `main.rs` | 1–2h     | Services spawn ✅                   | Yes     |
| 7   | HTTP Server Cleanup      | 1h       | HTTP works without cache ✅         | Yes     |
| 8   | WebSocket Infrastructure | 2–3h     | WS service compiles ✅              | Yes     |
| 9   | WebSocket Integration    | 1h       | `/ws` endpoint works ✅             | Yes     |
| 10  | Frontend WebSocket       | 2h       | Real-time updates work ✅           | Yes     |
| 11  | Testing & Validation     | 2–3h     | All tests pass ✅                   | Yes     |
| 12  | Pi Deployment            | 1–2h     | Production stable ✅                | Yes     |
| 13  | Cleanup & Docs           | 1h       | Committed to git ✅                 | Done    |

**Total Time**: 18–27 hours across multiple sessions.

---

## Rollback Plan

If something fails on the Pi, roll back quickly:

```sh
sudo systemctl stop home-assistant-rs
cp /opt/home-assistant-rs/home-assistant-rs.backup /opt/home-assistant-rs/home-assistant-rs
sudo systemctl start home-assistant-rs
sudo journalctl -u home-assistant-rs -f
```

**Backup before deployment** (add to Makefile deploy step):

```sh
ssh ludovic@Gholam.local "cp /opt/home-assistant-rs/home-assistant-rs /opt/home-assistant-rs/home-assistant-rs.backup"
```

**Database rollback** if schema migration fails:

```sh
make restore
# or
ssh ludovic@Gholam.local \
  "cp /opt/home-assistant-rs/database/backup/latest.db \
  /var/lib/home-assistant-rs/database/home_assistant.db"
```

---

## Final Summary

This refactor will:

- ✅ Save 300 KB–800 KB RAM (cache removal offsets event bus overhead).
- ✅ Deliver ~150× faster frontend updates (<100 ms vs. 15 s).
- ✅ Reduce bandwidth ~96% (1 MB/hour vs. 24 MB/hour).
- ✅ Fix dual-write race conditions and persist switch state.
- ✅ Enable parallel event processing and easier extensions.
- ✅ Improve maintainability with clearer service boundaries.

Costs and risks:

- ⚠️ 18–27 hours of engineering effort (+~1,100 LOC).
- ⚠️ Slightly higher complexity (more services to manage).
- ⚠️ Requires comprehensive testing prior to deployment.
