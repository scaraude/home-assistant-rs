import { writable } from 'svelte/store';
import type {
  DeviceInfo,
  DeviceState,
  SensorReading,
  StorageBreakdown,
  SwitchDevice,
} from '../api';

interface SensorStoreState {
  devices: DeviceInfo[];
  readings: SensorReading[];
  latestTimestamp: Date | null;
  rangeHours: number;
  loaded: boolean;
  graphSeries: Record<string, GraphSeriesWindow[]>;
}

interface SwitchStoreState {
  devices: SwitchDevice[];
  byId: Record<string, SwitchDevice>;
  loaded: boolean;
}

interface DataCacheState {
  sensors: SensorStoreState;
  switches: SwitchStoreState;
  deviceStates: Record<string, DeviceState>;
  storage: {
    breakdown: StorageBreakdown | null;
    loaded: boolean;
  };
}

interface GraphSeriesWindow {
  bucketSeconds: number;
  start: number;
  end: number;
  readings: SensorReading[];
}

const createInitialState = (): DataCacheState => ({
  sensors: {
    devices: [],
    readings: [],
    latestTimestamp: null,
    rangeHours: 24,
    loaded: false,
    graphSeries: {},
  },
  switches: {
    devices: [],
    byId: {},
    loaded: false,
  },
  deviceStates: {},
  storage: {
    breakdown: null,
    loaded: false,
  },
});

function sortReadings(readings: SensorReading[]): SensorReading[] {
  return [...readings].sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
}

function mergeGraphWindows(
  existing: GraphSeriesWindow[],
  nextWindow: GraphSeriesWindow,
): GraphSeriesWindow[] {
  const filtered = existing.filter(
    (window) =>
      window.bucketSeconds !== nextWindow.bucketSeconds ||
      window.end < nextWindow.start ||
      window.start > nextWindow.end,
  );

  const merged = [...filtered, nextWindow].sort((a, b) => a.start - b.start);

  // Keep the newest 8 windows per sensor to cap memory usage.
  return merged.slice(Math.max(0, merged.length - 8));
}

function trimReadings(readings: SensorReading[], hours: number): SensorReading[] {
  if (hours <= 0) {
    return readings;
  }
  const cutoff = Date.now() - hours * 3600 * 1000;
  return readings.filter((reading) => reading.timestamp.getTime() >= cutoff);
}

function createDataCache() {
  const { subscribe, set, update } = writable<DataCacheState>(createInitialState());

  return {
    subscribe,
    reset() {
      set(createInitialState());
    },

    setSensors(devices: DeviceInfo[]) {
      update((state) => ({
        ...state,
        sensors: {
          ...state.sensors,
          devices,
        },
      }));
    },

    updateDeviceName(deviceId: string, name: string) {
      update((state) => {
        const existingSwitch = state.switches.byId[deviceId];
        const updatedSwitch = existingSwitch ? { ...existingSwitch, name } : null;

        return {
          ...state,
          sensors: {
            ...state.sensors,
            devices: state.sensors.devices.map((device) =>
              device.device_id === deviceId ? { ...device, name } : device,
            ),
          },
          switches: updatedSwitch
            ? {
              ...state.switches,
              byId: {
                ...state.switches.byId,
                [deviceId]: updatedSwitch,
              },
              devices: state.switches.devices.map((device) =>
                device.id === deviceId ? updatedSwitch : device,
              ),
            }
            : state.switches,
        };
      });
    },

    updateDeviceColor(deviceId: string, color: string) {
      update((state) => {
        const existingSwitch = state.switches.byId[deviceId];
        const updatedSwitch = existingSwitch ? { ...existingSwitch, color } : null;

        return {
          ...state,
          sensors: {
            ...state.sensors,
            devices: state.sensors.devices.map((device) =>
              device.device_id === deviceId ? { ...device, color } : device,
            ),
          },
          switches: updatedSwitch
            ? {
              ...state.switches,
              byId: {
                ...state.switches.byId,
                [deviceId]: updatedSwitch,
              },
              devices: state.switches.devices.map((device) =>
                device.id === deviceId ? updatedSwitch : device,
              ),
            }
            : state.switches,
        };
      });
    },

    setSensorReadings(readings: SensorReading[], latestTimestamp: Date | null, rangeHours: number) {
      const sorted = sortReadings(readings);
      update((state) => ({
        ...state,
        sensors: {
          ...state.sensors,
          readings: sorted,
          latestTimestamp,
          rangeHours,
          loaded: true,
        },
      }));
    },

    setGraphSeries(
      deviceId: string,
      bucketSeconds: number,
      start: number,
      end: number,
      readings: SensorReading[],
    ) {
      const sorted = sortReadings(readings);
      update((state) => {
        const existing = state.sensors.graphSeries[deviceId] || [];
        const nextWindow: GraphSeriesWindow = {
          bucketSeconds,
          start,
          end,
          readings: sorted,
        };
        return {
          ...state,
          sensors: {
            ...state.sensors,
            graphSeries: {
              ...state.sensors.graphSeries,
              [deviceId]: mergeGraphWindows(existing, nextWindow),
            },
          },
        };
      });
    },

    clearGraphSeries(deviceId?: string) {
      update((state) => {
        if (!deviceId) {
          return {
            ...state,
            sensors: {
              ...state.sensors,
              graphSeries: {},
            },
          };
        }

        const { [deviceId]: _removed, ...rest } = state.sensors.graphSeries;
        return {
          ...state,
          sensors: {
            ...state.sensors,
            graphSeries: rest,
          },
        };
      });
    },

    mergeSensorReading(reading: SensorReading) {
      update((state) => {
        const exists = state.sensors.readings.some(
          (item) =>
            item.device_id === reading.device_id &&
            item.timestamp.getTime() === reading.timestamp.getTime() &&
            item.type === reading.type,
        );

        const merged = exists
          ? state.sensors.readings
          : sortReadings([...state.sensors.readings, reading]);

        const latestTimestamp = state.sensors.latestTimestamp;
        const nextLatest = latestTimestamp
          ? latestTimestamp.getTime() >= reading.timestamp.getTime()
            ? latestTimestamp
            : reading.timestamp
          : reading.timestamp;

        return {
          ...state,
          sensors: {
            ...state.sensors,
            readings: trimReadings(merged, state.sensors.rangeHours),
            latestTimestamp: nextLatest,
          },
        };
      });
    },

    setSwitches(devices: SwitchDevice[]) {
      const byId = devices.reduce<Record<string, SwitchDevice>>((acc, device) => {
        acc[device.id] = device;
        return acc;
      }, {});

      update((state) => ({
        ...state,
        switches: {
          devices,
          byId,
          loaded: true,
        },
      }));
    },

    updateSwitchState(
      deviceId: string,
      updates: Partial<Pick<SwitchDevice, 'state' | 'link_quality' | 'battery_level' | 'last_seen'>>,
    ) {
      update((state) => {
        const existing = state.switches.byId[deviceId];
        if (!existing) {
          return state;
        }

        const updatedDevice: SwitchDevice = {
          ...existing,
          ...updates,
        };

        return {
          ...state,
          switches: {
            ...state.switches,
            byId: {
              ...state.switches.byId,
              [deviceId]: updatedDevice,
            },
            devices: state.switches.devices.map((device) =>
              device.id === deviceId ? updatedDevice : device,
            ),
          },
        };
      });
    },

    updateDeviceState(deviceId: string, updates: Partial<DeviceState>) {
      update((state) => {
        const existing = state.deviceStates[deviceId];
        const nextState: DeviceState = {
          device_id: deviceId,
          battery_level: updates.battery_level ?? existing?.battery_level ?? null,
          link_quality: updates.link_quality ?? existing?.link_quality ?? null,
          turbo_mode: updates.turbo_mode ?? existing?.turbo_mode ?? null,
          last_seen: updates.last_seen ?? existing?.last_seen ?? new Date(),
        };

        const newState: DataCacheState = {
          ...state,
          deviceStates: {
            ...state.deviceStates,
            [deviceId]: nextState,
          },
        };

        if (newState.switches.byId[deviceId]) {
          const updatedDevice: SwitchDevice = {
            ...newState.switches.byId[deviceId],
            battery_level: nextState.battery_level,
            link_quality: nextState.link_quality,
            last_seen: nextState.last_seen,
          };

          newState.switches = {
            ...newState.switches,
            byId: {
              ...newState.switches.byId,
              [deviceId]: updatedDevice,
            },
            devices: newState.switches.devices.map((device) =>
              device.id === deviceId ? updatedDevice : device,
            ),
          };
        }

        return newState;
      });
    },

    setStorageBreakdown(breakdown: StorageBreakdown) {
      update((state) => ({
        ...state,
        storage: {
          breakdown,
          loaded: true,
        },
      }));
    },
  };
}

export const dataCache = createDataCache();
