import { writable } from 'svelte/stores';
import type { SensorReading, LogEntry } from '../api';

/**
 * Client-side cache for sensor readings with delta tracking
 */
interface SensorCache {
  readings: SensorReading[];
  latestTimestamp: number | null;
  etag: string | null;
  lastFetch: number; // timestamp of last fetch
}

/**
 * Client-side cache for log data with line tracking
 */
interface LogCache {
  entries: LogEntry[];
  totalLines: number;
  etag: string | null;
  lastFetch: number;
}

/**
 * Complete cache state
 */
interface CacheState {
  sensors: SensorCache;
  logs: {
    [filename: string]: LogCache;
  };
}

const INITIAL_STATE: CacheState = {
  sensors: {
    readings: [],
    latestTimestamp: null,
    etag: null,
    lastFetch: 0,
  },
  logs: {},
};

/**
 * Create a writable store for caching API data
 */
function createDataCache() {
  const { subscribe, set, update } = writable<CacheState>(INITIAL_STATE);

  return {
    subscribe,

    /**
     * Update sensor readings cache
     */
    setSensorReadings: (
      readings: SensorReading[],
      latestTimestamp: number | null,
      etag: string | null,
      merge: boolean = false
    ) => {
      update((state) => {
        const newReadings = merge
          ? mergeReadings(state.sensors.readings, readings)
          : readings;

        return {
          ...state,
          sensors: {
            readings: newReadings,
            latestTimestamp: latestTimestamp ?? state.sensors.latestTimestamp,
            etag,
            lastFetch: Date.now(),
          },
        };
      });
    },

    /**
     * Update log cache for a specific file
     */
    setLogData: (
      filename: string,
      entries: LogEntry[],
      totalLines: number,
      etag: string | null,
      merge: boolean = false
    ) => {
      update((state) => {
        const existingCache = state.logs[filename];
        const newEntries = merge && existingCache
          ? [...existingCache.entries, ...entries]
          : entries;

        return {
          ...state,
          logs: {
            ...state.logs,
            [filename]: {
              entries: newEntries,
              totalLines,
              etag,
              lastFetch: Date.now(),
            },
          },
        };
      });
    },

    /**
     * Get latest timestamp for delta fetching
     */
    getLatestTimestamp: (): number | null => {
      let latestTimestamp: number | null = null;
      update((state) => {
        latestTimestamp = state.sensors.latestTimestamp;
        return state;
      });
      return latestTimestamp;
    },

    /**
     * Get total lines for a log file (for delta fetching)
     */
    getLogTotalLines: (filename: string): number | null => {
      let totalLines: number | null = null;
      update((state) => {
        totalLines = state.logs[filename]?.totalLines ?? null;
        return state;
      });
      return totalLines;
    },

    /**
     * Clear all cached data
     */
    clear: () => {
      set(INITIAL_STATE);
    },

    /**
     * Clear sensor cache only
     */
    clearSensors: () => {
      update((state) => ({
        ...state,
        sensors: INITIAL_STATE.sensors,
      }));
    },

    /**
     * Clear log cache for a specific file
     */
    clearLog: (filename: string) => {
      update((state) => {
        const { [filename]: _, ...remainingLogs } = state.logs;
        return {
          ...state,
          logs: remainingLogs,
        };
      });
    },
  };
}

/**
 * Merge new readings with existing ones, removing duplicates
 * Assumes readings have unique timestamp + sensor_id combinations
 */
function mergeReadings(
  existing: SensorReading[],
  newReadings: SensorReading[]
): SensorReading[] {
  // Create a map of existing readings by unique key
  const map = new Map<string, SensorReading>();

  for (const reading of existing) {
    const key = `${reading.sensor_id}-${reading.timestamp}`;
    map.set(key, reading);
  }

  // Add new readings (will overwrite if duplicate)
  for (const reading of newReadings) {
    const key = `${reading.sensor_id}-${reading.timestamp}`;
    map.set(key, reading);
  }

  // Convert back to array and sort by timestamp
  return Array.from(map.values()).sort((a, b) => a.timestamp - b.timestamp);
}

/**
 * Global data cache instance
 */
export const dataCache = createDataCache();
