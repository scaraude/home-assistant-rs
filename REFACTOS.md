# Refactoring & Improvement Backlog

> **Important**: Before refactoring any component, ensure it has sufficient test coverage to validate that the refactoring didn't introduce regressions.

## Progress Tracking

### 🔴 Critical Priority
- [ ] CRIT-1: Security - API Input Validation Framework
- [x] CRIT-2: Performance - Automation Rule Evaluation N+1 Query
- [x] CRIT-3: Memory Leak - Unbounded HashMap in Automation Service

### 🟠 High Priority
- [ ] HIGH-1: Code Quality - Extract HTTP Error Handling Pattern
- [x] HIGH-2: Architecture - Split routes.rs by Domain
- [ ] HIGH-3: Testing - HTTP Route Test Coverage
- [ ] HIGH-4: Frontend - Design System & Style Consolidation
- [ ] HIGH-5: Frontend - Split api.ts by Domain
- [ ] HIGH-6: Frontend - Organize Components by Feature

### 🟡 Medium Priority
- [ ] MED-1: Naming Improvements Across Codebase
- [ ] MED-2: Extract Magic Numbers to Constants
- [ ] MED-3: Refactor Device Capability Model
- [ ] MED-4: Refactor if-else Chain in Device Discovery
- [ ] MED-5: Remove Unused Methods from State Stores
- [ ] MED-6: Improve Error Context in Database Operations
- [ ] MED-7: Frontend - TypeScript Type Improvements
- [ ] MED-8: Add Frontend Component Tests
- [ ] MED-9: Makefile Cleanup & Organization

### 🟢 Low Priority
- [ ] LOW-1: Remove Compilation Warnings
- [ ] LOW-2: Add RwLock Poisoning Recovery Logging
- [ ] LOW-3: Implement Timestamps as DateTime Types
- [ ] LOW-4: Add Service Health Monitoring
- [ ] LOW-5: Add MQTT Device Whitelist
- [ ] LOW-6: Add API Request Retry Logic (Frontend)

### 🔵 Future / Nice-to-Have
- [ ] FUTURE-1: Integration Test Suite
- [ ] FUTURE-2: API Documentation (OpenAPI/Swagger)
- [ ] FUTURE-3: SQLite Encryption at Rest
- [ ] FUTURE-4: Optimize RwLock Usage in State Stores
- [ ] FUTURE-5: Event Filtering at Subscription Level

---

## Priority Legend
- 🔴 **Critical**: High impact on performance, security, or stability
- 🟠 **High**: Significant improvement to code quality or maintainability
- 🟡 **Medium**: Nice-to-have improvements, reduces technical debt
- 🟢 **Low**: Cosmetic or minor improvements

## Cost Estimation Scale
- **XS**: < 1 hour
- **S**: 1-2 hours
- **M**: 2-4 hours (half-day)
- **L**: 4-8 hours (full day)
- **XL**: 1-2 days
- **XXL**: 3+ days

---

## 🔴 Critical Priority

### CRIT-1: Security - API Input Validation Framework
**Cost**: L (4-8 hours)
**Priority**: 🔴 Critical

**Problem**:
- No validation on device name updates ([routes.rs:191](src/http/routes.rs#L191)) - could accept unlimited length strings
- Automation rule names/descriptions unchecked - potential DoS via large payloads
- Query parameters lack bounds (e.g., `hours` parameter could trigger massive DB queries)
- Exposes system to DoS attacks and unexpected behavior

**Solution**:
1. Create `src/http/validation.rs` module with validation helpers:
   ```rust
   pub fn validate_device_name(name: &str) -> Result<(), ValidationError>
   pub fn validate_hours_param(hours: Option<u32>) -> Result<u32, ValidationError>
   pub fn validate_rule_payload(rule: &AutomationRulePayload) -> Result<(), ValidationError>
   ```
2. Add validation to all endpoints before processing
3. Return `400 Bad Request` with descriptive error messages
4. Add tests for edge cases (empty strings, max lengths, special characters)

**Files to modify**:
- Create: [src/http/validation.rs](src/http/validation.rs)
- Modify: [src/http/routes.rs](src/http/routes.rs)
- Add tests: [src/http/tests.rs](src/http/tests.rs)

**Benefits**:
- Prevents DoS attacks via unbounded queries
- Improves error messages for clients
- Protects database from malformed inputs

---

### CRIT-2: Performance - Automation Rule Evaluation N+1 Query
**Cost**: M (2-4 hours)
**Priority**: 🔴 Critical

**Problem**:
In [automation.rs:evaluate_conditions](src/services/automation.rs#L200-220), each condition triggers a separate database query:
```rust
for condition in &rule.conditions {
    self.db.get_latest_reading_for_sensor(&condition.device_id)?  // ← N queries
}
```
For a rule with 5 conditions, this hits the database 5 times per evaluation. On every sensor update, all applicable rules are evaluated.

**Solution**:
1. Add batch query method to database layer:
   ```rust
   pub fn get_latest_readings_batch(&self, device_ids: &[String])
       -> Result<HashMap<String, SensorReading>>
   ```
2. Modify evaluation logic to:
   - Collect all device IDs from conditions
   - Single batch query
   - Evaluate conditions against cached results

**Files to modify**:
- [src/db/queries/sensor.rs](src/db/queries/sensor.rs) - add batch query
- [src/services/automation.rs](src/services/automation.rs) - refactor evaluation loop
- [src/db/tests.rs](src/db/tests.rs) - add batch query tests

**Expected Improvement**:
- 5x reduction in DB queries for typical rules
- Lower SQLite lock contention
- Faster automation response times on Pi Zero

---

### CRIT-3: Memory Leak - Unbounded HashMap in Automation Service
**Cost**: S (1-2 hours)
**Priority**: 🔴 Critical

**Problem**:
[automation.rs:25](src/services/automation.rs#L25) uses an unbounded HashMap for debounce tracking:
```rust
last_triggered: Arc<RwLock<HashMap<String, DateTime<Utc>>>>
```
When rules are deleted, their entries remain in memory forever. Over months of operation, this accumulates orphaned entries.

**Solution**:
1. Add cleanup in `delete_rule_from_db()`:
   ```rust
   pub fn delete_rule(&self, rule_id: i64) -> Result<()> {
       self.db.delete_rule(rule_id)?;
       self.last_triggered.write().remove(&rule_id.to_string());  // ← cleanup
       Ok(())
   }
   ```
2. Alternatively, implement periodic cleanup task (scan for non-existent rule IDs every hour)

**Files to modify**:
- [src/services/automation.rs](src/services/automation.rs)

**Benefits**:
- Prevents slow memory leak on long-running systems
- Keeps HashMap small and performant

---

## 🟠 High Priority

### HIGH-1: Code Quality - Extract HTTP Error Handling Pattern
**Cost**: S (1-2 hours)
**Priority**: 🟠 High

**Problem**:
In [routes.rs](src/http/routes.rs), this pattern repeats 15+ times:
```rust
match db.get_something() {
    Ok(data) => match serde_json::to_string(&data) {
        Ok(json) => json_response(json),
        Err(e) => internal_error_response(...)
    }
    Err(e) => internal_error_response(...)
}
```

**Solution**:
Create generic helper in [http/responses.rs](src/http/responses.rs):
```rust
pub fn serialize_or_error<T: Serialize>(
    data: T,
    error_context: &str
) -> Response<Body> {
    match serde_json::to_string(&data) {
        Ok(json) => json_response(json),
        Err(e) => {
            error!("Failed to serialize {}: {}", error_context, e);
            internal_error_response("Failed to serialize response")
        }
    }
}

// Usage:
db.get_all_sensors()
    .map(|sensors| serialize_or_error(sensors, "sensors list"))
    .unwrap_or_else(|e| internal_error_response(&e.to_string()))
```

**Files to modify**:
- [src/http/responses.rs](src/http/responses.rs) - add helper
- [src/http/routes.rs](src/http/routes.rs) - apply to all endpoints

**Benefits**:
- Reduces ~120 lines of boilerplate
- Consistent error logging
- Easier to modify error handling globally

---

### HIGH-2: Architecture - Split routes.rs by Domain
**Cost**: M (2-4 hours)
**Priority**: 🟠 High

**Problem**:
[routes.rs](src/http/routes.rs) is 1,301 lines containing 5 different domains:
- Sensor endpoints (readings, devices)
- Switch endpoints (commands, state)
- Automation endpoints (rules CRUD, executions)
- Logs endpoints (files, viewer)
- Network endpoints (process monitoring)

**Solution**:
Create domain modules:
```
src/http/routes/
├── mod.rs          # Router and module exports
├── sensors.rs      # Sensor-related endpoints
├── switches.rs     # Switch control endpoints
├── automation.rs   # Automation rule endpoints
├── logs.rs         # Log viewing endpoints
└── system.rs       # System monitoring endpoints
```

Each module exports its handlers:
```rust
// src/http/routes/sensors.rs
pub fn handle_get_sensors(db: &Database) -> Response<Body> { ... }
pub fn handle_get_readings(db: &Database, query: &str) -> Response<Body> { ... }
```

Main router delegates:
```rust
// src/http/routes/mod.rs
match path {
    "/api/sensors" => sensors::handle_get_sensors(db),
    "/api/readings" => sensors::handle_get_readings(db, query),
    ...
}
```

**Files to create**:
- [src/http/routes/mod.rs](src/http/routes/mod.rs)
- [src/http/routes/sensors.rs](src/http/routes/sensors.rs)
- [src/http/routes/switches.rs](src/http/routes/switches.rs)
- [src/http/routes/automation.rs](src/http/routes/automation.rs)
- [src/http/routes/logs.rs](src/http/routes/logs.rs)
- [src/http/routes/system.rs](src/http/routes/system.rs)

**Files to remove**:
- [src/http/routes.rs](src/http/routes.rs) (split into modules)

**Benefits**:
- Easier navigation and maintenance
- Clear separation of concerns
- Smaller files for code review

---

### HIGH-3: Testing - HTTP Route Test Coverage
**Cost**: L (4-8 hours)
**Priority**: 🟠 High

**Problem**:
- Zero tests for HTTP handlers ([routes.rs](src/http/routes.rs))
- Only database layer tested (92 tests in [db/tests.rs](src/db/tests.rs))
- No validation that endpoints return correct status codes
- No testing of query parameter parsing edge cases

**Solution**:
Create comprehensive test suite:
```rust
// src/http/routes/tests.rs
#[tokio::test]
async fn test_get_sensors_success() { ... }

#[tokio::test]
async fn test_get_sensors_db_error() { ... }

#[tokio::test]
async fn test_update_device_invalid_name() { ... }

#[tokio::test]
async fn test_get_readings_invalid_hours_param() { ... }
```

**Test Categories**:
1. Happy path (200 OK with valid data)
2. Error cases (404, 500 with proper messages)
3. Input validation (400 Bad Request)
4. Query parameter parsing edge cases
5. Content-Type headers
6. JSON serialization edge cases

**Target Coverage**: All public endpoint handlers

**Files to create**:
- [src/http/routes/tests.rs](src/http/routes/tests.rs) (or per-module tests)

**Benefits**:
- Confidence in refactoring routes
- Regression detection
- Documentation of expected behavior

---

### HIGH-4: Frontend - Design System & Style Consolidation
**Cost**: L (4-8 hours)
**Priority**: 🟠 High

**Problem**:
- Duplicated styles across [SensorCard.svelte](frontend/src/lib/SensorCard.svelte), [PresenceCard.svelte](frontend/src/lib/PresenceCard.svelte), [EnergyMeterCard.svelte](frontend/src/lib/EnergyMeterCard.svelte)
- Inconsistent spacing, colors, shadows across components
- No centralized design tokens (colors, spacing, typography)
- Harder to maintain consistent UI

**Solution**:
1. Create design system module:
   ```
   frontend/src/lib/design-system/
   ├── tokens.ts           # Design tokens (colors, spacing, shadows)
   ├── Button.svelte       # Reusable button component
   ├── Card.svelte         # Base card component
   ├── Badge.svelte        # Status badge
   └── Typography.svelte   # Text components
   ```

2. Define design tokens:
   ```typescript
   // tokens.ts
   export const colors = {
     primary: '#3b82f6',
     success: '#10b981',
     warning: '#f59e0b',
     danger: '#ef4444',
     background: '#1f2937',
     cardBg: '#374151',
     text: '#f3f4f6'
   };

   export const spacing = {
     xs: '0.25rem',
     sm: '0.5rem',
     md: '1rem',
     lg: '1.5rem',
     xl: '2rem'
   };

   export const shadows = {
     sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
     md: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
     lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1)'
   };
   ```

3. Create base Card component:
   ```svelte
   <!-- Card.svelte -->
   <script lang="ts">
     import { shadows } from './tokens';
     export let title: string;
     export let shadow: keyof typeof shadows = 'md';
   </script>

   <div class="card" style="box-shadow: {shadows[shadow]}">
     <h3>{title}</h3>
     <slot />
   </div>
   ```

4. Refactor existing cards to use design system components

**Files to create**:
- [frontend/src/lib/design-system/tokens.ts](frontend/src/lib/design-system/tokens.ts)
- [frontend/src/lib/design-system/Card.svelte](frontend/src/lib/design-system/Card.svelte)
- [frontend/src/lib/design-system/Button.svelte](frontend/src/lib/design-system/Button.svelte)
- [frontend/src/lib/design-system/Badge.svelte](frontend/src/lib/design-system/Badge.svelte)

**Files to refactor**:
- [frontend/src/lib/SensorCard.svelte](frontend/src/lib/SensorCard.svelte)
- [frontend/src/lib/PresenceCard.svelte](frontend/src/lib/PresenceCard.svelte)
- [frontend/src/lib/SwitchCard.svelte](frontend/src/lib/SwitchCard.svelte)
- [frontend/src/lib/EnergyMeterCard.svelte](frontend/src/lib/EnergyMeterCard.svelte)

**Benefits**:
- Unified visual design
- Easier to update styles globally
- Reduced CSS duplication (~30% reduction estimated)
- Better maintainability

---

### HIGH-5: Frontend - Split api.ts by Domain
**Cost**: M (2-4 hours)
**Priority**: 🟠 High

**Problem**:
[api.ts](frontend/src/lib/api.ts) contains all API calls in one file (~300+ lines estimated):
- Device APIs
- Sensor reading APIs
- Switch control APIs
- Automation rule APIs
- System monitoring APIs
- Log viewing APIs

**Solution**:
Split into domain modules:
```
frontend/src/lib/api/
├── index.ts        # Re-exports all APIs
├── devices.ts      # Device management
├── sensors.ts      # Sensor readings
├── switches.ts     # Switch control
├── automation.ts   # Automation rules
├── system.ts       # System monitoring
└── logs.ts         # Log viewing
```

Each module groups related APIs:
```typescript
// sensors.ts
export async function getSensors(): Promise<Device[]> { ... }
export async function getReadings(params: ReadingsParams): Promise<Reading[]> { ... }
export async function getDeviceState(deviceId: string): Promise<DeviceState> { ... }
```

Main index re-exports:
```typescript
// index.ts
export * from './devices';
export * from './sensors';
export * from './switches';
export * from './automation';
export * from './system';
export * from './logs';
```

**Files to create**:
- [frontend/src/lib/api/index.ts](frontend/src/lib/api/index.ts)
- [frontend/src/lib/api/devices.ts](frontend/src/lib/api/devices.ts)
- [frontend/src/lib/api/sensors.ts](frontend/src/lib/api/sensors.ts)
- [frontend/src/lib/api/switches.ts](frontend/src/lib/api/switches.ts)
- [frontend/src/lib/api/automation.ts](frontend/src/lib/api/automation.ts)
- [frontend/src/lib/api/system.ts](frontend/src/lib/api/system.ts)
- [frontend/src/lib/api/logs.ts](frontend/src/lib/api/logs.ts)

**Files to remove**:
- [frontend/src/lib/api.ts](frontend/src/lib/api.ts) (split into modules)

**Files to update**:
- All components importing from `'./api'` (update to `'./api/index'` or specific modules)

**Benefits**:
- Better code organization
- Easier to find related API calls
- Clearer domain boundaries
- Reduced merge conflicts

---

### HIGH-6: Frontend - Organize Components by Feature
**Cost**: M (2-4 hours)
**Priority**: 🟠 High

**Problem**:
All components in flat [frontend/src/lib/](frontend/src/lib/) directory:
- SensorCard.svelte
- PresenceCard.svelte
- EnergyMeterCard.svelte
- SwitchCard.svelte
- GraphModal.svelte
- AutomationRulePanel.svelte
- SystemMonitor.svelte
- TimeAgo.svelte
- etc.

Hard to navigate when looking for feature-specific components.

**Solution**:
Organize by feature:
```
frontend/src/lib/
├── devices/
│   ├── SensorCard.svelte
│   ├── PresenceCard.svelte
│   ├── EnergyMeterCard.svelte
│   ├── SwitchCard.svelte
│   └── DeviceList.svelte
├── automation/
│   ├── AutomationRulePanel.svelte
│   ├── RuleForm.svelte
│   └── RuleExecutionLog.svelte
├── system/
│   ├── SystemMonitor.svelte
│   └── LogViewer.svelte
├── graphs/
│   ├── GraphModal.svelte
│   └── GraphConfig.svelte
├── shared/
│   └── TimeAgo.svelte
├── stores/
│   ├── dataCache.ts
│   ├── graphConfig.ts
│   └── websocket.ts
├── api/
│   └── [domain modules]
└── design-system/
    └── [design tokens and components]
```

**Files to move**: All existing components

**Files to update**:
- [App.svelte](frontend/src/App.svelte) - update imports
- All components with cross-component imports

**Benefits**:
- Clear feature boundaries
- Easier to find related components
- Better scalability as features grow

---

## 🟡 Medium Priority

### MED-1: Naming Improvements Across Codebase
**Cost**: M (2-4 hours)
**Priority**: 🟡 Medium

**Problem**:
Misleading or unclear naming in multiple places:

**Backend**:
- `get_all_sensors()` → actually returns `get_all_sensor_devices()` (devices, not readings)
- `TemperatureReading` → should be `SensorReading` (handles all sensor types)
- `has_sensor_data()` → unclear, should be `has_sensor_capability()`

**Frontend**:
- `TemperatureGraph` component → actually `SensorGraph` (shows any sensor type)
- `field: 'temperature' | 'humidity' | 'battery' | ...` → should use typed unions

**Solution**:

1. Backend renames:
   ```rust
   // src/db/queries/device.rs
   - pub fn get_all_sensors() -> Result<Vec<Device>>
   + pub fn get_all_sensor_devices() -> Result<Vec<Device>>

   // src/models/mqtt.rs
   - pub struct TemperatureReading
   + pub struct SensorReading

   // src/models/device.rs
   - pub fn has_sensor_data(&self, sensor_type: &SensorType) -> bool
   + pub fn has_sensor_capability(&self, sensor_type: &SensorType) -> bool
   ```

2. Frontend renames:
   ```typescript
   // Rename component file
   - TemperatureGraph.svelte
   + SensorGraph.svelte

   // Define proper type unions
   type TempSensorField = 'temperature' | 'humidity';
   type PresenceSensorField = 'presence' | 'illumination';
   type DeviceStateField = 'battery' | 'link_quality';

   type SensorField = TempSensorField | PresenceSensorField | DeviceStateField;
   ```

**Files to modify**:
- [src/db/queries/device.rs](src/db/queries/device.rs)
- [src/models/mqtt.rs](src/models/mqtt.rs)
- [src/models/device.rs](src/models/device.rs)
- [src/http/routes.rs](src/http/routes.rs) (update calls)
- [frontend/src/lib/TemperatureGraph.svelte](frontend/src/lib/TemperatureGraph.svelte) → rename file
- [frontend/src/lib/api.ts](frontend/src/lib/api.ts) (update types)

**Benefits**:
- Clearer code intent
- Easier onboarding for new developers
- Reduced confusion during debugging

---

### MED-2: Extract Magic Numbers to Constants
**Cost**: S (1-2 hours)
**Priority**: 🟡 Medium

**Problem**:
Hardcoded values scattered throughout codebase with no explanation:

```rust
// src/mqtt/client.rs:38
.set_max_packet_size(1024 * 1024, 1024 * 1024)  // Why 1MB?

// src/main.rs:66
let (event_tx, _) = broadcast::channel(1000);  // Why 1000?

// src/main.rs:86
hyper::server::conn::Http::new()
    .http1_pipeline_flush(true)
    .http1_keep_alive(true)
    .http1_max_buf_size(Some(20));  // Why 20?

// src/services/automation.rs:46
const DEBOUNCE_SECONDS: i64 = 60;  // Only one with explanation!
```

**Solution**:
Create module-level constants with documentation:

```rust
// src/mqtt/client.rs
/// Maximum MQTT packet size (1MB). Chosen to accommodate large device
/// discovery messages from zigbee2mqtt with many devices.
const MQTT_MAX_PACKET_SIZE: usize = 1024 * 1024;

// src/main.rs
/// Event bus capacity. Size chosen to buffer ~10 seconds of events at
/// peak load (100 events/sec from MQTT + sensor updates).
const EVENT_BUS_CAPACITY: usize = 1000;

/// HTTP connection buffer size. Small value optimized for Pi Zero 2W
/// memory constraints while handling typical API payloads (<5KB).
const HTTP_MAX_BUF_SIZE: usize = 20;

// src/services/automation.rs
/// Automation rule debounce window (60 seconds). Prevents rules from
/// triggering multiple times for the same sensor event (e.g., temperature
/// fluctuating around threshold).
const DEBOUNCE_SECONDS: i64 = 60;
```

**Files to modify**:
- [src/mqtt/client.rs](src/mqtt/client.rs)
- [src/main.rs](src/main.rs)
- [src/services/automation.rs](src/services/automation.rs)

**Benefits**:
- Easier to tune performance
- Documents why values were chosen
- Single place to update configuration

---

### MED-3: Refactor Device Capability Model
**Cost**: L (4-8 hours)
**Priority**: 🟡 Medium

**Problem**:
Current model ([device.rs](src/models/device.rs)) assumes one capability per device:
```rust
pub enum DeviceCapability {
    Sensor { sensor_type: SensorType },
    Commander { commander_type: CommanderType },
}
```

But real devices can have multiple capabilities:
- Switch + Router (Zigbee coordinator)
- Energy Meter + Router
- Switch + Energy Meter + Router

**Solution**:
Change to capability array model:

```rust
// New capability types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Capability {
    Sensor(SensorType),
    Commander(CommanderType),
    Router { turbo_mode: bool },  // Zigbee routing capability
}

// Updated Device model
pub struct Device {
    pub id: String,
    pub name: String,
    pub mqtt_topic: String,
    pub capabilities: Vec<Capability>,  // ← Changed from single capability
    pub available_fields: Vec<String>,  // ← New: fields sent by z2m
}

// Helper methods
impl Device {
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn is_sensor(&self) -> bool {
        self.capabilities.iter().any(|c| matches!(c, Capability::Sensor(_)))
    }

    pub fn is_router(&self) -> bool {
        self.capabilities.iter().any(|c| matches!(c, Capability::Router { .. }))
    }
}
```

**Database Migration**:
```sql
-- Add migration script
ALTER TABLE devices ADD COLUMN capabilities TEXT NOT NULL DEFAULT '[]';
ALTER TABLE devices ADD COLUMN available_fields TEXT NOT NULL DEFAULT '[]';

-- Migrate existing data
UPDATE devices
SET capabilities = json_array(
  json_object('type', capability_type, 'subtype', capability_subtype)
);

-- Drop old columns after migration
ALTER TABLE devices DROP COLUMN capability_type;
ALTER TABLE devices DROP COLUMN capability_subtype;
```

**Files to modify**:
- [src/models/device.rs](src/models/device.rs) - update model
- [src/db/schema.rs](src/db/schema.rs) - update schema
- [src/db/queries/device.rs](src/db/queries/device.rs) - update queries
- [src/mqtt/handlers/device.rs](src/mqtt/handlers/device.rs) - update discovery logic
- Add migration script: [migrations/004_multi_capability.sql](migrations/004_multi_capability.sql)

**Testing Requirements**:
- Test device with multiple capabilities
- Test queries filtering by capability
- Test backward compatibility with existing devices

**Benefits**:
- Accurately models real device capabilities
- Supports router devices properly
- Allows future expansion (e.g., OTA update capability)

---

### MED-4: Refactor if-else Chain in Device Discovery
**Cost**: S (1-2 hours)
**Priority**: 🟡 Medium

**Problem**:
[mqtt/handlers/device.rs:51-75](src/mqtt/handlers/device.rs#L51-75) has deep if-else chain:

```rust
let capability = if msg.has_sensor_data(&SensorType::TempHumidity) {
    DeviceCapability::Sensor { sensor_type: SensorType::TempHumidity }
} else if msg.has_sensor_data(&SensorType::Presence) {
    DeviceCapability::Sensor { sensor_type: SensorType::Presence }
} else if msg.has_sensor_data(&SensorType::EnergyMeter) {
    DeviceCapability::Sensor { sensor_type: SensorType::EnergyMeter }
} else if msg.has_commander_data(&CommanderType::Switch) || msg.has_switch_config_hint() {
    DeviceCapability::Commander { commander_type: CommanderType::Switch }
} else {
    error!(mqtt_topic = %mqtt_topic, "Cannot determine device type");
    return None;
};
```

**Solution**:
Use match with capability detection:

```rust
impl DeviceMqttMessage {
    /// Detect all capabilities from MQTT message
    pub fn detect_capabilities(&self) -> Vec<DeviceCapability> {
        let mut capabilities = Vec::new();

        // Check sensor capabilities
        for sensor_type in &[SensorType::TempHumidity, SensorType::Presence, SensorType::EnergyMeter] {
            if self.has_sensor_data(sensor_type) {
                capabilities.push(DeviceCapability::Sensor { sensor_type: *sensor_type });
            }
        }

        // Check commander capabilities
        for commander_type in &[CommanderType::Switch] {
            if self.has_commander_data(commander_type) || self.has_switch_config_hint() {
                capabilities.push(DeviceCapability::Commander { commander_type: *commander_type });
            }
        }

        capabilities
    }
}

// Usage in device discovery:
let capabilities = msg.detect_capabilities();
if capabilities.is_empty() {
    error!(mqtt_topic = %mqtt_topic, "No capabilities detected");
    return None;
}
```

**Files to modify**:
- [src/models/mqtt.rs](src/models/mqtt.rs) - add detection method
- [src/mqtt/handlers/device.rs](src/mqtt/handlers/device.rs) - use detection method

**Benefits**:
- More maintainable (easy to add new device types)
- Supports multi-capability devices
- Clearer logic flow

---

### MED-5: Remove Unused Methods from State Stores
**Cost**: XS (< 1 hour)
**Priority**: 🟡 Medium

**Problem**:
[src/state/device.rs](src/state/device.rs) has unused methods prefixed with `_`:

```rust
pub fn _get_all_states(&self) -> Vec<(String, DeviceState)> { ... }  // Line 50
pub fn _remove_state(&self, device_id: &str) { ... }                 // Line 59
pub fn _clear(&self) { ... }                                          // Line 64
```

These are never called in the codebase.

**Solution**:
1. Search for any usage of these methods
2. If truly unused, delete them
3. If needed for debugging, move to `#[cfg(test)]` block:

```rust
#[cfg(test)]
impl DeviceStateStore {
    pub fn get_all_states(&self) -> Vec<(String, DeviceState)> { ... }
    pub fn clear(&self) { ... }
}
```

**Files to modify**:
- [src/state/device.rs](src/state/device.rs)
- [src/state/switch.rs](src/state/switch.rs) (check for similar unused methods)

**Benefits**:
- Cleaner public API
- Less code to maintain
- Clear test-only methods

---

### MED-6: Improve Error Context in Database Operations
**Cost**: M (2-4 hours)
**Priority**: 🟡 Medium

**Problem**:
Error messages lose context when propagated:

```rust
// src/http/routes.rs:405
db.get_device_by_mqtt_topic(&topic)
    .map_err(|e| format!("Database error: {}", e))?
```

When this fails, you see "Database error: no rows" but don't know:
- Which table was queried
- What topic was searched
- Which endpoint triggered it

**Solution**:
Add contextual error wrapping:

```rust
// src/db/queries/device.rs
pub fn get_device_by_mqtt_topic(&self, topic: &str) -> Result<Device> {
    self.conn.query_row(...)
        .map_err(|e| {
            error!(
                mqtt_topic = %topic,
                error = %e,
                "Failed to get device by MQTT topic"
            );
            e
        })
}

// src/http/routes.rs
db.get_device_by_mqtt_topic(&topic)
    .map_err(|e| format!("Failed to find device with topic '{}': {}", topic, e))?
```

**Files to modify**:
- [src/db/queries/device.rs](src/db/queries/device.rs)
- [src/db/queries/sensor.rs](src/db/queries/sensor.rs)
- [src/db/queries/automation.rs](src/db/queries/automation.rs)
- [src/http/routes.rs](src/http/routes.rs)

**Benefits**:
- Easier debugging in production
- Better error messages for clients
- Clearer logs

---

### MED-7: Frontend - TypeScript Type Improvements
**Cost**: M (2-4 hours)
**Priority**: 🟡 Medium

**Problem**:
Union types too broad, losing type safety:

```typescript
// From original REFACTOS.md
field: 'temperature' | 'humidity' | 'battery' | 'link_quality' | 'presence' | 'illumination';
capability_subtype: 'temp_humidity' | 'presence' | 'switch';
```

**Solution**:
Create specific type hierarchies:

```typescript
// types/sensors.ts
export type TempHumiditySensorField = 'temperature' | 'humidity';
export type PresenceSensorField = 'presence' | 'illumination';
export type EnergyMeterField = 'power' | 'voltage' | 'current' | 'energy';

export type SensorField =
  | TempHumiditySensorField
  | PresenceSensorField
  | EnergyMeterField;

export type DeviceStateField = 'battery' | 'link_quality';

// Capability subtypes
export enum CapabilitySubType {
  TempHumidity = 'temp_humidity',
  Presence = 'presence',
  EnergyMeter = 'energy_meter',
  Switch = 'switch'
}

// Type guards
export function isTempHumidityField(field: string): field is TempHumiditySensorField {
  return field === 'temperature' || field === 'humidity';
}

export function isPresenceField(field: string): field is PresenceSensorField {
  return field === 'presence' || field === 'illumination';
}
```

Usage in components:
```typescript
<script lang="ts">
  import type { TempHumiditySensorField } from './types/sensors';

  export let field: TempHumiditySensorField;

  // TypeScript now knows field can only be 'temperature' or 'humidity'
</script>
```

**Files to create**:
- [frontend/src/lib/types/sensors.ts](frontend/src/lib/types/sensors.ts)
- [frontend/src/lib/types/devices.ts](frontend/src/lib/types/devices.ts)

**Files to modify**:
- [frontend/src/lib/api.ts](frontend/src/lib/api.ts) - use new types
- All components using device/sensor types

**Benefits**:
- Better IntelliSense/autocomplete
- Compile-time error detection
- Self-documenting code

---

### MED-8: Add Frontend Component Tests
**Cost**: L (4-8 hours)
**Priority**: 🟡 Medium

**Problem**:
Zero test coverage for frontend components and stores.

**Solution**:
Set up Vitest + Testing Library and add tests for critical components:

```typescript
// frontend/src/lib/SwitchCard.test.ts
import { render, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
import SwitchCard from './SwitchCard.svelte';

describe('SwitchCard', () => {
  it('renders switch name and state', () => {
    const { getByText } = render(SwitchCard, {
      props: {
        device: { id: '1', name: 'Living Room Light', state: 'ON' }
      }
    });

    expect(getByText('Living Room Light')).toBeInTheDocument();
    expect(getByText('ON')).toBeInTheDocument();
  });

  it('calls API when toggle clicked', async () => {
    const executeCommand = vi.fn();
    vi.mock('./api', () => ({ executeCommand }));

    const { getByRole } = render(SwitchCard, { ... });
    await fireEvent.click(getByRole('button'));

    expect(executeCommand).toHaveBeenCalledWith('1', 'TOGGLE');
  });
});
```

**Testing Priorities**:
1. SwitchCard (state management + API calls)
2. AutomationRulePanel (complex form logic)
3. dataCache store (WebSocket event handling)
4. graphConfig store (data transformations)

**Files to create**:
- [frontend/vitest.config.ts](frontend/vitest.config.ts)
- [frontend/src/lib/SwitchCard.test.ts](frontend/src/lib/SwitchCard.test.ts)
- [frontend/src/lib/AutomationRulePanel.test.ts](frontend/src/lib/AutomationRulePanel.test.ts)
- [frontend/src/lib/stores/dataCache.test.ts](frontend/src/lib/stores/dataCache.test.ts)
- [frontend/src/lib/stores/graphConfig.test.ts](frontend/src/lib/stores/graphConfig.test.ts)

**Files to modify**:
- [frontend/package.json](frontend/package.json) - add test deps and script

**Benefits**:
- Confidence in refactoring
- Catch regressions early
- Better component API design

---

### MED-9: Makefile Cleanup & Organization
**Cost**: S (1-2 hours)
**Priority**: 🟡 Medium

**Problem**:
From original REFACTOS.md:
- Inconsistent naming (build-frontend vs quick-deploy)
- No clear sections
- Missing deploy-config target

**Solution**:
Reorganize Makefile with clear sections:

```makefile
# =============================================================================
# Configuration
# =============================================================================
include .env.deploy
export

RUST_TARGET ?= aarch64-unknown-linux-gnu
DEPLOY_DIR ?= /opt/home-automation-rs

# =============================================================================
# Build Targets
# =============================================================================
.PHONY: build
build: ## Cross-compile backend for ARM64
	cross build --release --target $(RUST_TARGET)

.PHONY: build-frontend
build-frontend: ## Build Svelte frontend
	cd frontend && npm run build

.PHONY: build-all
build-all: build build-frontend ## Build both backend and frontend

# =============================================================================
# Deployment Targets
# =============================================================================
.PHONY: deploy-backend
deploy-backend: build transfer-binary restart ## Build and deploy backend only

.PHONY: deploy-frontend
deploy-frontend: build-frontend transfer-frontend ## Build and deploy frontend only

.PHONY: deploy-config
deploy-config: ## Generate and transfer configuration files
	@echo "Generating configs/pi.env..."
	@cat configs/pi.env.template | envsubst > configs/pi.env
	scp configs/pi.env $(PI_USER)@$(PI_HOST):$(DEPLOY_DIR)/configs/

.PHONY: deploy-full
deploy-full: setup-dependencies build-all transfer-all setup-services restart ## Full deployment (first-time setup)

# =============================================================================
# Transfer Targets
# =============================================================================
.PHONY: transfer-binary
transfer-binary: ## Transfer compiled binary to Pi
	scp target/$(RUST_TARGET)/release/home-automation-rs $(PI_USER)@$(PI_HOST):$(DEPLOY_DIR)/

.PHONY: transfer-frontend
transfer-frontend: ## Transfer built frontend to Pi
	rsync -av --delete frontend/build/ $(PI_USER)@$(PI_HOST):$(DEPLOY_DIR)/static/

.PHONY: transfer-all
transfer-all: transfer-binary transfer-frontend transfer-config ## Transfer everything

# =============================================================================
# Service Management
# =============================================================================
.PHONY: start
start: ## Start the service
	ssh $(PI_USER)@$(PI_HOST) "sudo systemctl start home-automation-rs"

.PHONY: stop
stop: ## Stop the service
	ssh $(PI_USER)@$(PI_HOST) "sudo systemctl stop home-automation-rs"

.PHONY: restart
restart: ## Restart the service
	ssh $(PI_USER)@$(PI_HOST) "sudo systemctl restart home-automation-rs"

# =============================================================================
# Development Targets
# =============================================================================
.PHONY: test
test: ## Run all tests
	cargo test

.PHONY: test-watch
test-watch: ## Run tests in watch mode
	cargo watch -x test

.PHONY: check
check: ## Check for compilation errors without building
	cargo check

# =============================================================================
# Helpers
# =============================================================================
.PHONY: help
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'
```

**Files to modify**:
- [Makefile](Makefile)
- [.env.deploy](configs/.env.deploy) - ensure all vars documented

**Benefits**:
- Easier to find commands
- Consistent naming convention
- Self-documenting with `make help`

---

## 🟢 Low Priority

### LOW-1: Remove Compilation Warnings
**Cost**: S (1-2 hours)
**Priority**: 🟢 Low

**Problem**:
Compilation produces warnings (need to run build to identify specific warnings).

**Solution**:
1. Run `cargo build` and `cargo clippy` to identify all warnings
2. Fix each warning category:
   - Unused imports
   - Unused variables (prefix with `_` if intentionally unused)
   - Deprecated API usage
   - Dead code
   - Missing documentation on public items

3. Run `cargo clippy -- -D warnings` to treat warnings as errors

**Files to modify**: TBD (depends on warnings found)

**Benefits**:
- Cleaner build output
- Catches potential bugs early
- Better code quality

---

### LOW-2: Add RwLock Poisoning Recovery Logging
**Cost**: XS (< 1 hour)
**Priority**: 🟢 Low

**Problem**:
[db/connection.rs](src/db/connection.rs) silently recovers from poisoned mutexes:

```rust
pub fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            // Silent recovery - no indication which operation panicked!
            poisoned.into_inner()
        }
    }
}
```

**Solution**:
Add logging to track poisoning events:

```rust
pub fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            error!(
                "Mutex was poisoned, recovering. This indicates a thread panicked while holding the lock. \
                 Check logs for panic details."
            );
            poisoned.into_inner()
        }
    }
}
```

**Files to modify**:
- [src/db/connection.rs](src/db/connection.rs)

**Benefits**:
- Visibility into threading issues
- Easier debugging of rare race conditions

---

### LOW-3: Implement Timestamps as DateTime Types
**Cost**: M (2-4 hours)
**Priority**: 🟢 Low

**Problem**:
From original REFACTOS.md: "timestamps should be Date type in the app (front and back)"

Currently using `i64` Unix timestamps everywhere.

**Solution**:

**Backend** (Rust has `chrono::DateTime`):
```rust
// Already using DateTime<Utc> in many places!
// Just need consistency across all timestamp fields

pub struct SensorReading {
    pub device_id: String,
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub timestamp: DateTime<Utc>,  // ← Already done!
}
```

**Frontend** (use Date objects):
```typescript
// Before
interface Reading {
  timestamp: number;  // Unix timestamp
}

// After
interface Reading {
  timestamp: Date;
}

// API response parsing
export async function getReadings(): Promise<Reading[]> {
  const response = await fetch('/api/readings');
  const data = await response.json();
  return data.map(r => ({
    ...r,
    timestamp: new Date(r.timestamp * 1000)  // Convert Unix to Date
  }));
}
```

**Files to modify**:
- [frontend/src/lib/api.ts](frontend/src/lib/api.ts) - parse timestamps
- [frontend/src/lib/types/sensors.ts](frontend/src/lib/types/sensors.ts) - update types
- All components using timestamps

**Benefits**:
- Better type safety
- Easier date manipulation
- Clearer intent

**Note**: Backend already uses `DateTime<Utc>` extensively, so this is mostly a frontend concern.

---

### LOW-4: Add Service Health Monitoring
**Cost**: L (4-8 hours)
**Priority**: 🟢 Low

**Problem**:
No monitoring of service health:
- Services could silently stop processing events
- No indication if event bus is lagging
- No deadletter queue for failed events

**Solution**:
Add health monitoring service:

```rust
// src/services/health_monitor.rs
pub struct HealthMonitor {
    event_rx: broadcast::Receiver<SystemEvent>,
    last_event_time: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    metrics_tx: mpsc::Sender<HealthMetric>,
}

impl HealthMonitor {
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                // Track event processing
                Ok(event) = self.event_rx.recv() => {
                    self.record_event(&event);
                }

                // Periodic health checks (every 30s)
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    self.check_service_health();
                }
            }
        }
    }

    fn check_service_health(&self) {
        // Check if services haven't processed events recently
        let now = Utc::now();
        for (service, last_time) in self.last_event_time.read().unwrap().iter() {
            let elapsed = now.signed_duration_since(*last_time);
            if elapsed > Duration::from_secs(300) {  // 5 min threshold
                warn!(
                    service = %service,
                    elapsed_seconds = elapsed.num_seconds(),
                    "Service may be stalled"
                );
            }
        }
    }
}
```

Add health endpoint:
```rust
// src/http/routes.rs
"/api/health" => {
    let metrics = health_monitor.get_metrics();
    json_response(serde_json::to_string(&metrics).unwrap())
}
```

**Files to create**:
- [src/services/health_monitor.rs](src/services/health_monitor.rs)

**Files to modify**:
- [src/main.rs](src/main.rs) - spawn health monitor
- [src/http/routes.rs](src/http/routes.rs) - add health endpoint

**Benefits**:
- Early detection of service issues
- Visibility into system health
- Supports automated monitoring

---

### LOW-5: Add MQTT Device Whitelist
**Cost**: M (2-4 hours)
**Priority**: 🟢 Low

**Problem**:
MQTT bridge accepts all devices from `zigbee2mqtt/#` topic without validation. Attacker with MQTT access could register malicious devices.

**Solution**:
Add device whitelist configuration:

```rust
// configs/devices.toml
[allowed_devices]
models = [
    "SNZB-02",      # Sonoff temp/humidity
    "SNZB-03",      # Sonoff motion
    "TS0121",       # Tuya smart plug
]

# Optional: specific device IDs
ids = [
    "0x00124b001f3c5678",
]
```

Validation in device discovery:
```rust
// src/mqtt/handlers/device.rs
pub fn validate_device(msg: &DeviceMqttMessage, config: &DeviceConfig) -> Result<()> {
    // Check model whitelist
    if let Some(model) = &msg.model {
        if !config.allowed_models.contains(model) {
            return Err(anyhow!("Device model '{}' not in whitelist", model));
        }
    }

    // Check device ID whitelist (if configured)
    if !config.allowed_ids.is_empty() {
        if !config.allowed_ids.contains(&msg.friendly_name) {
            return Err(anyhow!("Device ID '{}' not in whitelist", msg.friendly_name));
        }
    }

    Ok(())
}
```

**Files to create**:
- [configs/devices.toml.example](configs/devices.toml.example)

**Files to modify**:
- [src/mqtt/handlers/device.rs](src/mqtt/handlers/device.rs)
- [src/main.rs](src/main.rs) - load device config

**Benefits**:
- Security against rogue devices
- Explicit device authorization
- Audit trail of allowed devices

---

### LOW-6: Add API Request Retry Logic (Frontend)
**Cost**: S (1-2 hours)
**Priority**: 🟢 Low

**Problem**:
[frontend/src/lib/api.ts](frontend/src/lib/api.ts) has no retry logic for failed requests. Network blips cause complete failures.

**Solution**:
Add exponential backoff retry wrapper:

```typescript
// api/retry.ts
interface RetryOptions {
  maxAttempts: number;
  baseDelayMs: number;
  maxDelayMs: number;
}

export async function fetchWithRetry<T>(
  url: string,
  options: RequestInit = {},
  retryOpts: RetryOptions = {
    maxAttempts: 3,
    baseDelayMs: 1000,
    maxDelayMs: 10000
  }
): Promise<T> {
  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= retryOpts.maxAttempts; attempt++) {
    try {
      const response = await fetch(url, options);

      if (!response.ok) {
        // Don't retry client errors (400-499)
        if (response.status >= 400 && response.status < 500) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return await response.json();
    } catch (error) {
      lastError = error as Error;

      if (attempt < retryOpts.maxAttempts) {
        // Exponential backoff: 1s, 2s, 4s, ...
        const delay = Math.min(
          retryOpts.baseDelayMs * Math.pow(2, attempt - 1),
          retryOpts.maxDelayMs
        );

        console.warn(`Request failed (attempt ${attempt}/${retryOpts.maxAttempts}), retrying in ${delay}ms...`, error);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
  }

  throw lastError;
}
```

Usage:
```typescript
// api/sensors.ts
export async function getSensors(): Promise<Device[]> {
  return fetchWithRetry<Device[]>('/api/sensors');
}
```

**Files to create**:
- [frontend/src/lib/api/retry.ts](frontend/src/lib/api/retry.ts)

**Files to modify**:
- [frontend/src/lib/api/index.ts](frontend/src/lib/api/index.ts) - use retry wrapper

**Benefits**:
- More resilient to network issues
- Better user experience
- Reduces failed requests

---

## 🔵 Nice-to-Have / Future

### FUTURE-1: Integration Test Suite
**Cost**: XL (1-2 days)
**Priority**: 🔵 Future

**Description**:
End-to-end tests for full event flow:
1. Mock MQTT broker publishes sensor reading
2. Verify event propagates through system
3. Verify database write
4. Verify WebSocket broadcast
5. Verify automation rule triggers

**Requires**:
- Test MQTT broker setup
- Test database fixtures
- WebSocket client for testing

---

### FUTURE-2: API Documentation (OpenAPI/Swagger)
**Cost**: M (2-4 hours)
**Priority**: 🔵 Future

**Description**:
Generate OpenAPI spec from code or manually document:
- All endpoints
- Request/response schemas
- Status codes
- Query parameters

Serve at `/api/docs`.

---

### FUTURE-3: SQLite Encryption at Rest
**Cost**: L (4-8 hours)
**Priority**: 🔵 Future

**Description**:
Use SQLCipher to encrypt sensor data at rest. Requires:
- Key management strategy
- Migration for existing databases
- Performance testing on Pi Zero

---

### FUTURE-4: Optimize RwLock Usage in State Stores
**Cost**: M (2-4 hours)
**Priority**: 🔵 Future

**Description**:
Evaluate if RwLock is necessary or if Mutex would be simpler:
- Profile lock contention
- Measure read vs write frequency
- Consider lock-free alternatives (dashmap, etc.)

---

### FUTURE-5: Event Filtering at Subscription Level
**Cost**: L (4-8 hours)
**Priority**: 🔵 Future

**Description**:
Allow services to subscribe to specific event types only:
```rust
let sensor_events = event_rx.filter(|e| matches!(e, SystemEvent::SensorReading { .. }));
```

Reduces unnecessary event processing in each service.

---

## Summary Statistics

### By Priority
- 🔴 Critical: 3 tickets
- 🟠 High: 6 tickets
- 🟡 Medium: 9 tickets
- 🟢 Low: 6 tickets
- 🔵 Future: 5 tickets

**Total**: 29 tickets

### By Cost
- XS (< 1h): 2 tickets
- S (1-2h): 6 tickets
- M (2-4h): 11 tickets
- L (4-8h): 7 tickets
- XL (1-2d): 2 tickets
- XXL (3+d): 1 ticket

### Recommended Roadmap

**Phase 1 - Security & Performance (1-2 weeks)**:
1. CRIT-1: API Input Validation (security)
2. CRIT-2: Automation N+1 Query (performance)
3. CRIT-3: Memory Leak Fix (stability)
4. HIGH-1: Extract HTTP Error Pattern (code quality)

**Phase 2 - Code Organization (1-2 weeks)**:
1. HIGH-2: Split routes.rs by Domain
2. HIGH-5: Split api.ts by Domain
3. HIGH-6: Organize Components by Feature
4. MED-9: Makefile Cleanup

**Phase 3 - Testing & Quality (2-3 weeks)**:
1. HIGH-3: HTTP Route Tests
2. MED-8: Frontend Component Tests
3. HIGH-4: Design System (reduces duplication)
4. MED-1: Naming Improvements

**Phase 4 - Technical Debt (1-2 weeks)**:
1. MED-3: Device Capability Model Refactor
2. MED-4: Refactor if-else Chain
3. MED-5: Remove Unused Methods
4. MED-6: Improve Error Context
5. LOW-1: Remove Compilation Warnings

**Phase 5 - Polish & Future (ongoing)**:
- LOW-* tickets as time permits
- FUTURE-* tickets for major version bumps
