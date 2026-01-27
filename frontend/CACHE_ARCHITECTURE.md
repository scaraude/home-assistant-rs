# Frontend Cache Architecture

## Overview

This document describes the proposed domain-based cache architecture for the home-automation-rs frontend. The goal is to create a clear separation between different domains (sensors, energy, switches, automation) with each domain managing its own data and UI state.

## Current Architecture Problems

### Shared Global State Issues

1. **No clear domain boundaries**: Components reach into global stores and filter data themselves, leading to:
   - Duplicated filtering logic
   - Tight coupling between components and data structure
   - Difficult to reason about data flow

## Proposed Architecture

### Core Principles

1. **Domain-based organization**: Each feature domain has its own cache
2. **Single source of truth per domain**: Components access only their domain's cache
3. **Centralized data fetching**: Raw API data stored in shared layer, domain caches derive from it
4. **WebSocket events route to appropriate domain**: Event handlers update the correct domain cache
5. **Lazy initialization**: Domain caches initialize on first access, not eagerly

### Directory Structure

```
frontend/src/lib/
├── core/                          # Shared infrastructure
│   ├── api/                       # API client functions (existing)
│   ├── websocket/
│   │   ├── client.ts              # WebSocket connection management
│   │   ├── events.ts              # Event type definitions
│   │   └── router.ts              # Routes events to domain caches
│   └── stores/
│       └── rawDataStore.ts        # Raw API responses (readings, devices)
│
├── sensors/                       # Temperature/Humidity domain
│   ├── cache.ts                   # SensorDomainCache
│   ├── types.ts                   # Domain-specific types
│   ├── components/
│   │   ├── SensorListPanel.svelte
│   │   ├── CompactSensorCard.svelte
│   │   └── PresenceCard.svelte
│   └── index.ts                   # Public exports
│
├── energy/                        # Energy meter domain
│   ├── cache.ts                   # EnergyDomainCache
│   ├── types.ts
│   ├── components/
│   │   ├── EnergyListPanel.svelte
│   │   └── EnergyMeterCard.svelte
│   └── index.ts
│
├── switches/                      # Switch/Commander domain
│   ├── cache.ts                   # SwitchDomainCache
│   ├── types.ts
│   ├── components/
│   │   ├── SwitchListPanel.svelte
│   │   └── SwitchCard.svelte
│   └── index.ts
│
├── automation/                    # Automation rules domain
│   ├── cache.ts                   # AutomationDomainCache
│   ├── types.ts
│   ├── components/
│   │   └── ...
│   └── index.ts
│
├── graphs/                        # Shared charting (receives data from domains)
│   ├── UnifiedChart.svelte
│   ├── GraphToolbar.svelte
│   └── types.ts
│
└── shared/                        # Shared UI components
    ├── design-system/
    └── utils/
```

### Domain Cache Interface

Each domain cache follows a consistent interface:

```typescript
// Example: sensors/cache.ts

import { writable, derived, type Readable } from "svelte/store";
import type { SensorReading, DeviceInfo } from "../core/api";

// Domain-specific UI state
export interface SensorUIConfig {
  deviceId: string;
  color: string;
  visible: boolean;
}

// Domain cache state
interface SensorCacheState {
  // Data
  devices: DeviceInfo[]; // Only temp/humidity/presence sensors
  readings: SensorReading[]; // Only readings for this domain's devices

  // UI State
  sensorConfigs: SensorUIConfig[];
  selectedMetric: "temperature" | "humidity" | "both";
  timeRange: "24h" | "1w" | "1m" | "1y";

  // Loading state
  initialized: boolean;
  loading: boolean;
  error: string | null;
}

interface SensorDomainCache {
  // Readable store
  subscribe: Readable<SensorCacheState>["subscribe"];

  // Derived stores for common access patterns
  visibleSensors: Readable<SensorUIConfig[]>;
  latestReadings: Readable<Map<string, SensorReading>>;

  // Actions
  initialize(): Promise<void>; // Fetch initial data
  refresh(force?: boolean): Promise<void>; // Re-fetch data
  reset(): void; // Clear UI state (not data)

  // Data mutations (from WebSocket)
  mergeReading(reading: SensorReading): void;
  updateDeviceState(deviceId: string, state: Partial<DeviceState>): void;

  // UI state mutations
  toggleSensor(deviceId: string): void;
  isolateSensor(deviceId: string): void;
  showAllSensors(): void;
  setSensorColor(deviceId: string, color: string): void;
  setMetric(metric: "temperature" | "humidity" | "both"): void;
  setTimeRange(range: "24h" | "1w" | "1m" | "1y"): void;
}
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Backend                                      │
│  (MQTT → Event Bus → WebSocket Broadcast)                                │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         WebSocket Client                                  │
│  - Connects to /ws                                                       │
│  - Parses SystemEvent                                                    │
│  - Routes to EventRouter                                                 │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Event Router                                     │
│  - sensor_reading → SensorCache OR EnergyCache (based on type)           │
│  - switch_state → SwitchCache                                            │
│  - device_state → All relevant domain caches                             │
│  - automation_* → AutomationCache                                        │
└──────┬──────────────────┬──────────────────┬──────────────────┬─────────┘
       │                  │                  │                  │
       ▼                  ▼                  ▼                  ▼
┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────┐
│SensorCache │    │EnergyCache │    │SwitchCache │    │Automation  │
│            │    │            │    │            │    │Cache       │
│ - devices  │    │ - devices  │    │ - devices  │    │ - rules    │
│ - readings │    │ - readings │    │ - states   │    │ - logs     │
│ - UI state │    │ - UI state │    │ - UI state │    │ - UI state │
└──────┬─────┘    └──────┬─────┘    └──────┬─────┘    └──────┬─────┘
       │                  │                  │                  │
       ▼                  ▼                  ▼                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           Svelte Components                               │
│  - Subscribe to their domain cache only                                  │
│  - Use $derived for computed values                                      │
│  - Call cache actions for mutations                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

### Implementation Example: SensorCache

```typescript
// frontend/src/lib/sensors/cache.ts

import { writable, derived } from "svelte/store";
import { fetchSensors, fetchReadings } from "../core/api";
import type { DeviceInfo, SensorReading } from "../core/api";

const STORAGE_KEY = "homeAutomation:sensorCache";
const RECOMMENDED_COLORS = [
  "#3b82f6",
  "#ef4444",
  "#10b981",
  "#f59e0b",
  "#8b5cf6",
  "#ec4899",
  "#14b8a6",
  "#f97316",
];

export interface SensorUIConfig {
  deviceId: string;
  color: string;
  visible: boolean;
}

interface SensorCacheState {
  devices: DeviceInfo[];
  readings: SensorReading[];
  sensorConfigs: SensorUIConfig[];
  selectedMetric: "temperature" | "humidity" | "both";
  timeRange: "24h" | "1w" | "1m" | "1y";
  initialized: boolean;
  loading: boolean;
  error: string | null;
}

const TIME_RANGE_HOURS = {
  "24h": 24,
  "1w": 168,
  "1m": 720,
  "1y": 8760,
};

// Filter: only temp_humidity and presence sensors
function isSensorDevice(device: DeviceInfo): boolean {
  return device.capabilities.some(
    (cap) =>
      cap.type === "sensor" &&
      (cap.sensor_type === "temp_humidity" || cap.sensor_type === "presence"),
  );
}

function createSensorCache() {
  const initialState: SensorCacheState = {
    devices: [],
    readings: [],
    sensorConfigs: [],
    selectedMetric: "temperature",
    timeRange: "24h",
    initialized: false,
    loading: false,
    error: null,
  };

  // Load persisted UI preferences
  const persisted = loadFromStorage();
  const { subscribe, set, update } = writable<SensorCacheState>({
    ...initialState,
    ...persisted,
  });

  // Derived stores
  const visibleSensors = derived({ subscribe }, ($state) =>
    $state.sensorConfigs.filter((s) => s.visible),
  );

  const latestReadings = derived({ subscribe }, ($state) => {
    const map = new Map<string, SensorReading>();
    for (const reading of $state.readings) {
      const existing = map.get(reading.device_id);
      if (!existing || reading.timestamp > existing.timestamp) {
        map.set(reading.device_id, reading);
      }
    }
    return map;
  });

  const visibleReadings = derived({ subscribe }, ($state) => {
    const visibleIds = new Set(
      $state.sensorConfigs.filter((s) => s.visible).map((s) => s.deviceId),
    );
    return $state.readings.filter((r) => visibleIds.has(r.device_id));
  });

  function loadFromStorage(): Partial<SensorCacheState> {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        return {
          selectedMetric: parsed.selectedMetric,
          timeRange: parsed.timeRange,
          sensorConfigs: parsed.sensorConfigs || [],
        };
      }
    } catch (e) {
      console.warn("Failed to load sensor cache from storage:", e);
    }
    return {};
  }

  function persistToStorage(state: SensorCacheState) {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          selectedMetric: state.selectedMetric,
          timeRange: state.timeRange,
          sensorConfigs: state.sensorConfigs.map((s) => ({
            deviceId: s.deviceId,
            color: s.color,
            // Don't persist visibility - always start with all visible
          })),
        }),
      );
    } catch (e) {
      console.warn("Failed to persist sensor cache:", e);
    }
  }

  return {
    subscribe,
    visibleSensors,
    latestReadings,
    visibleReadings,

    async initialize(): Promise<void> {
      update((s) => ({ ...s, loading: true, error: null }));

      try {
        const [allDevices, readingsResult] = await Promise.all([
          fetchSensors(),
          fetchReadings(undefined, TIME_RANGE_HOURS[initialState.timeRange]),
        ]);

        // Filter to only sensor devices
        const devices = allDevices.filter(isSensorDevice);
        const deviceIds = new Set(devices.map((d) => d.device_id));

        // Filter readings to only this domain's devices
        const readings = readingsResult.readings.filter(
          (r) => deviceIds.has(r.device_id) && r.type !== "energy_meter",
        );

        update((state) => {
          // Preserve color preferences, initialize new sensors
          const existingColors = new Map(
            state.sensorConfigs.map((s) => [s.deviceId, s.color]),
          );

          const sensorConfigs: SensorUIConfig[] = devices.map((d, idx) => ({
            deviceId: d.device_id,
            color:
              existingColors.get(d.device_id) ||
              RECOMMENDED_COLORS[idx % RECOMMENDED_COLORS.length],
            visible: true,
          }));

          const newState = {
            ...state,
            devices,
            readings,
            sensorConfigs,
            initialized: true,
            loading: false,
          };

          persistToStorage(newState);
          return newState;
        });
      } catch (e) {
        update((s) => ({
          ...s,
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load sensors",
        }));
      }
    },

    async refresh(force = false): Promise<void> {
      // Implementation similar to initialize, but respects `force` flag
      // for cache invalidation
    },

    reset(): void {
      update((state) => ({
        ...state,
        sensorConfigs: state.sensorConfigs.map((s) => ({
          ...s,
          visible: true,
        })),
      }));
    },

    // WebSocket event handlers
    mergeReading(reading: SensorReading): void {
      // Only accept readings for devices in this domain
      update((state) => {
        if (!state.devices.some((d) => d.device_id === reading.device_id)) {
          return state; // Not our device
        }
        if (reading.type === "energy_meter") {
          return state; // Not our reading type
        }

        const exists = state.readings.some(
          (r) =>
            r.device_id === reading.device_id &&
            r.timestamp.getTime() === reading.timestamp.getTime(),
        );

        if (exists) return state;

        const readings = [...state.readings, reading].sort(
          (a, b) => a.timestamp.getTime() - b.timestamp.getTime(),
        );

        // Trim to time range
        const cutoff =
          Date.now() - TIME_RANGE_HOURS[state.timeRange] * 3600 * 1000;
        const trimmed = readings.filter((r) => r.timestamp.getTime() >= cutoff);

        return { ...state, readings: trimmed };
      });
    },

    // UI state mutations
    toggleSensor(deviceId: string): void {
      update((state) => ({
        ...state,
        sensorConfigs: state.sensorConfigs.map((s) =>
          s.deviceId === deviceId ? { ...s, visible: !s.visible } : s,
        ),
      }));
    },

    isolateSensor(deviceId: string): void {
      update((state) => ({
        ...state,
        sensorConfigs: state.sensorConfigs.map((s) => ({
          ...s,
          visible: s.deviceId === deviceId,
        })),
      }));
    },

    showAllSensors(): void {
      update((state) => ({
        ...state,
        sensorConfigs: state.sensorConfigs.map((s) => ({
          ...s,
          visible: true,
        })),
      }));
    },

    setSensorColor(deviceId: string, color: string): void {
      update((state) => {
        const newState = {
          ...state,
          sensorConfigs: state.sensorConfigs.map((s) =>
            s.deviceId === deviceId ? { ...s, color } : s,
          ),
        };
        persistToStorage(newState);
        return newState;
      });
    },

    setMetric(metric: "temperature" | "humidity" | "both"): void {
      update((state) => {
        const newState = { ...state, selectedMetric: metric };
        persistToStorage(newState);
        return newState;
      });
    },

    setTimeRange(timeRange: "24h" | "1w" | "1m" | "1y"): void {
      update((state) => {
        const newState = { ...state, timeRange };
        persistToStorage(newState);
        return newState;
      });
      // Trigger data refresh for new time range
      this.refresh(true);
    },
  };
}

export const sensorCache = createSensorCache();
```

### WebSocket Event Router

```typescript
// frontend/src/lib/core/websocket/router.ts

import { sensorCache } from "../../sensors/cache";
import { energyCache } from "../../energy/cache";
import { switchCache } from "../../switches/cache";
import type { SystemEvent } from "./events";

export function routeEvent(event: SystemEvent): void {
  switch (event.event) {
    case "sensor_reading":
      // Route based on reading type
      if (event.reading.type === "energy_meter") {
        energyCache.mergeReading(event.reading);
      } else {
        sensorCache.mergeReading(event.reading);
      }
      break;

    case "switch_state":
      switchCache.updateState(event.device_id, event.state);
      break;

    case "device_state":
      // Device state updates go to all relevant caches
      sensorCache.updateDeviceState(event.device_id, event);
      energyCache.updateDeviceState(event.device_id, event);
      switchCache.updateDeviceState(event.device_id, event);
      break;

    case "device_discovered":
      // New device - each cache checks if it's relevant
      sensorCache.maybeAddDevice(event);
      energyCache.maybeAddDevice(event);
      switchCache.maybeAddDevice(event);
      break;

    // ... other event types
  }
}
```

### Component Usage Example

```svelte
<!-- frontend/src/lib/sensors/components/SensorListPanel.svelte -->
<script lang="ts">
  import { sensorCache } from '../cache';
  import CompactSensorCard from './CompactSensorCard.svelte';
  import PresenceCard from './PresenceCard.svelte';

  // Subscribe to domain cache - no filtering needed!
  const sensorConfigs = $derived($sensorCache.sensorConfigs);
  const latestReadings = $derived($sensorCache.latestReadings);

  function isPresenceSensor(deviceId: string): boolean {
    const reading = latestReadings.get(deviceId);
    return reading?.type === 'presence';
  }
</script>

<div class="sensor-list-panel">
  {#each sensorConfigs as config (config.deviceId)}
    {@const reading = latestReadings.get(config.deviceId)}
    {#if isPresenceSensor(config.deviceId)}
      <PresenceCard sensor={config} latestReading={reading} />
    {:else}
      <CompactSensorCard sensor={config} latestReading={reading} />
    {/if}
  {/each}
</div>
```

### View Integration

```svelte
<!-- frontend/src/routes/SensorsView.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { sensorCache } from '../lib/sensors/cache';
  import SensorListPanel from '../lib/sensors/components/SensorListPanel.svelte';
  import UnifiedGraphPanel from '../lib/graphs/UnifiedGraphPanel.svelte';

  const loading = $derived($sensorCache.loading);
  const error = $derived($sensorCache.error);
  const initialized = $derived($sensorCache.initialized);

  onMount(() => {
    if (!initialized) {
      sensorCache.initialize();
    }
  });
</script>

{#if loading}
  <LoadingSpinner />
{:else if error}
  <ErrorMessage message={error} onRetry={() => sensorCache.refresh(true)} />
{:else}
  <div class="sensor-view">
    <UnifiedGraphPanel
      readings={$sensorCache.visibleReadings}
      sensors={$sensorCache.visibleSensors}
      metric={$sensorCache.selectedMetric}
    />
    <SensorListPanel />
  </div>
{/if}
```

## Migration Strategy

### Phase 1: Quick Fix (Current - TICKET-006)

- [x] Add `graphConfig.reset()` method
- [x] Call reset on view mount
- [x] Filter energy meters in SensorListPanel

### Phase 2: Create Domain Caches

1. Create `sensors/cache.ts` with full implementation
2. Create `energy/cache.ts` mirroring sensor cache for energy meters
3. Update WebSocket client to use event router
4. Migrate SensorsView to use sensorCache
5. Migrate ConsommationsView to use energyCache

### Phase 3: Migrate Remaining Domains

1. Create `switches/cache.ts`
2. Create `automation/cache.ts`
3. Migrate CommanderView
4. Migrate automation components

### Phase 4: Cleanup

1. Remove old `graphConfig` store (or repurpose for shared UI preferences)
2. Remove old `dataCache` store
3. Update tests
4. Update documentation

## Benefits

1. **Clear domain boundaries**: Components know exactly where their data comes from
2. **No cross-domain pollution**: Energy meters can't appear in sensor view
3. **Simplified components**: No filtering logic needed in UI components
4. **Better type safety**: Each cache has domain-specific types
5. **Easier testing**: Domain caches can be tested in isolation
6. **Predictable data flow**: WebSocket → Router → Domain Cache → Component
7. **Preserved preferences**: Each domain persists its own UI state

## Considerations

1. **Initial complexity**: More files and structure to manage
2. **Duplication**: Some cache logic is similar across domains (mitigated by shared utilities)
3. **Migration effort**: Requires updating all views and components
4. **Bundle size**: Minimal impact - caches are small, tree-shaking handles unused code
