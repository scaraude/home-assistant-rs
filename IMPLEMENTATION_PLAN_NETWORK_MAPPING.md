# Network Map Implementation Plan

---

## Feature 1: Network Topology Visualization

### Goal

Display Zigbee mesh as interactive graph with devices, parent-child connections, and link quality.

### Backend

**1. Extend Device model** (`src/models/device.rs`)

```rust
pub is_bridge: bool,                    // type: "Router" in networkmap
pub parent_device_id: Option<String>,   // null for coordinator
```

**2. Create bridge handler** (`src/mqtt/handlers/bridge.rs`)

- Subscribe to `zigbee2mqtt/bridge/response/networkmap`
- Subscribe to `zigbee2mqtt/bridge/event` (pairing: `device_joined`, `device_interview`)
- Parse response → update `is_bridge` + `parent_device_id` on devices
- Emit `SystemEvent::NetworkTopologyUpdated`

**3. Route bridge topics** (`src/mqtt/event_loop.rs`)

- Route `zigbee2mqtt/bridge/response/*` and `zigbee2mqtt/bridge/event` to bridge handler

**4. Add events** (`src/events/mod.rs`)

```rust
NetworkTopologyUpdated,
DevicePairing { ieee_addr: String, status: PairingStatus }
```

**5. Add API routes** (`src/http/routes.rs`)

- `GET /api/network/topology` → devices + computed edges
- `POST /api/network/refresh` → publish `{"type": "raw", "routes": true}` to `zigbee2mqtt/bridge/request/networkmap`

**6. Topology response structure**

```rust
pub struct NetworkTopology {
    pub devices: Vec<Device>,
    pub edges: Vec<NetworkEdge>,  // computed from parent_device_id
}

pub struct NetworkEdge {
    pub source_id: String,
    pub target_id: String,
    pub link_quality: Option<u8>,
}
```

### Frontend

**1. Install dependency**

```bash
npm install @xyflow/svelte@^1.4.1
```

**2. Create store** (`frontend/src/lib/stores/networkTopology.ts`)

```typescript
interface NetworkState {
  devices: Device[];
  edges: NetworkEdge[];
  positions: Record<string, { x: number; y: number }>; // localStorage
  loading: boolean;
}
```

**3. Create components**
| File | Purpose |
|------|---------|
| `frontend/src/lib/network/NetworkMapView.svelte` | View + toolbar (refresh button with warning) |
| `frontend/src/lib/network/NetworkGraph.svelte` | Svelte Flow wrapper |
| `frontend/src/lib/network/DeviceNode.svelte` | Node with icon, name, LQI badge |

**4. Node styling**

- Icon by capability (sensor, switch, etc.)
- Bridge: blue border if `is_bridge === true`
- Coordinator: gold border (no parent)
- Reuse: `EditableDeviceName`, `StatusBadge`

**5. Edge styling** (by LQI)

- Green (#22c55e): > 100
- Yellow (#eab308): 50-100
- Red (#ef4444): < 50

**6. Interactions**

- Drag → save position to localStorage
- Click → show detail panel
- Refresh → warning + trigger networkmap

**7. Add route** (`frontend/src/App.svelte`)

- `/network` route
- Handle `network_topology_updated` WebSocket event

### API calls (`frontend/src/lib/api.ts`)

```typescript
fetchNetworkTopology(): Promise<NetworkTopology>
refreshNetworkMap(): Promise<void>
```

---

## Feature 2: Turbo Mode Toggle

### Goal

Detect and toggle `turbo_mode` on devices that support it.

### Backend

**1. Extend DeviceState** (`src/models/device.rs`)

```rust
pub turbo_mode: Option<bool>,  // None if device doesn't support it
```

**2. Parse from MQTT payload** (`src/mqtt/handlers/device.rs`)

- Extract `turbo_mode` from device messages
- Update DeviceState

**3. Add API route** (`src/http/routes.rs`)

- `POST /api/devices/{id}/set` → publish `{"turbo_mode": true/false}` to `zigbee2mqtt/{device}/set`

### Frontend

**1. Show in device detail panel** (`DeviceNode.svelte` or detail panel)

- Toggle switch if `turbo_mode !== null`
- Call `setDeviceOption(deviceId, "turbo_mode", value)`

**2. API call** (`frontend/src/lib/api.ts`)

```typescript
setDeviceOption(deviceId: string, option: string, value: any): Promise<void>
```

---

## Files Summary

| Feature    | Backend Files                                                                 | Frontend Files                                                                                                    |
| ---------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Network    | `device.rs`, `bridge.rs` (new), `event_loop.rs`, `events/mod.rs`, `routes.rs` | `NetworkMapView.svelte`, `NetworkGraph.svelte`, `DeviceNode.svelte`, `networkTopology.ts`, `api.ts`, `App.svelte` |
| Turbo Mode | `device.rs`, `device.rs` handler, `routes.rs`                                 | `DeviceNode.svelte`, `api.ts`                                                                                     |

## Dependencies

- `@xyflow/svelte` ^1.4.1

Backend Verification Checklist ✅

✅ Device model extended with network topology fields
✅ Database schema migrated (backward compatible)
✅ All device queries updated to handle new columns
✅ Bridge MQTT handler processes networkmap responses
✅ Network topology and pairing events defined
✅ API routes registered and tested for compilation
✅ Cargo check passes with only harmless dead_code warnings
