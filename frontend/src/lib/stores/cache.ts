import { writable } from 'svelte/store';
import type { SensorReading, LogEntry } from '../api';

/**
 * Minimal cache store for delta API requests
 * Tracks timestamps and line numbers to enable incremental updates
 */

interface SensorCache {
  readings: SensorReading[];
  latestTimestamp: number;
}

interface LogCache {
  entries: LogEntry[];
  totalLines: number;
}

interface CacheState {
  sensors: SensorCache;
  logs: Record<string, LogCache>;
}

function createCache() {
  const initialState: CacheState = {
    sensors: {
      readings: [],
      latestTimestamp: 0,
    },
    logs: {},
  };

  const { subscribe, update } = writable<CacheState>(initialState);

  return {
    subscribe,

    // Sensor methods
    setSensorReadings(readings: SensorReading[], latestTimestamp: number) {
      update((state) => ({
        ...state,
        sensors: { readings, latestTimestamp },
      }));
    },

    mergeSensorReadings(newReadings: SensorReading[], latestTimestamp: number) {
      update((state) => ({
        ...state,
        sensors: {
          readings: [...state.sensors.readings, ...newReadings],
          latestTimestamp,
        },
      }));
    },

    getSensorTimestamp(): number {
      let timestamp = 0;
      this.subscribe((state) => {
        timestamp = state.sensors.latestTimestamp;
      })();
      return timestamp;
    },

    // Log methods
    setLogEntries(filename: string, entries: LogEntry[], totalLines: number) {
      update((state) => ({
        ...state,
        logs: {
          ...state.logs,
          [filename]: { entries, totalLines },
        },
      }));
    },

    mergeLogEntries(filename: string, newEntries: LogEntry[], totalLines: number) {
      update((state) => {
        const existing = state.logs[filename] || { entries: [], totalLines: 0 };
        return {
          ...state,
          logs: {
            ...state.logs,
            [filename]: {
              entries: [...existing.entries, ...newEntries],
              totalLines,
            },
          },
        };
      });
    },

    getLogTotalLines(filename: string): number {
      let totalLines = 0;
      this.subscribe((state) => {
        totalLines = state.logs[filename]?.totalLines || 0;
      })();
      return totalLines;
    },

    // Clear methods
    clearSensors() {
      update((state) => ({
        ...state,
        sensors: { readings: [], latestTimestamp: 0 },
      }));
    },

    clearLog(filename: string) {
      update((state) => {
        const { [filename]: _, ...remainingLogs } = state.logs;
        return {
          ...state,
          logs: remainingLogs,
        };
      });
    },

    clearAll() {
      update(() => initialState);
    },
  };
}

export const cache = createCache();
