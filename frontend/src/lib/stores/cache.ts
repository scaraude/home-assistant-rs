import { writable } from 'svelte/store';
import type { SensorReading, LogEntry, TimeRange } from '../api';
import type { LogFile } from '../websocket';

/**
 * Cache store for log entries with timestamp-based tracking
 * Supports lazy loading per tab and WebSocket delta updates
 */

interface SensorCache {
  readings: SensorReading[];
  latestTimestamp: Date | null;
}

interface LogCache {
  entries: LogEntry[];
  /** The time range that was fetched (e.g., "24h", "1w") */
  timeRange: TimeRange;
  /** Timestamp of the most recent entry (for deduplication) */
  latestEntryTimestamp: string | null;
  /** Whether this log has been loaded at least once */
  loaded: boolean;
}

interface CacheState {
  sensors: SensorCache;
  logs: Record<string, LogCache>;
}

/** Map log file enum values to filenames */
const LOG_FILE_TO_FILENAME: Record<LogFile, string> = {
  system_monitor: 'system_monitor.log',
  process_monitor: 'process_monitor.log',
  top_cpu_consumers: 'top_cpu_consumers.log',
  top_ram_consumers: 'top_ram_consumers.log',
};

function createCache() {
  const initialState: CacheState = {
    sensors: {
      readings: [],
      latestTimestamp: null,
    },
    logs: {},
  };

  const { subscribe, update } = writable<CacheState>(initialState);

  /** Extract timestamp from a log entry */
  function getEntryTimestamp(entry: LogEntry): string {
    if ('cpu_usage' in entry) return entry.timestamp; // SystemMonitorEntry
    if ('status' in entry) return entry.timestamp; // ProcessMonitorEntry
    if ('rank' in entry) return entry.timestamp; // TopConsumerEntry
    return '';
  }

  return {
    subscribe,

    // Sensor methods
    setSensorReadings(readings: SensorReading[], latestTimestamp: Date | null) {
      update((state) => ({
        ...state,
        sensors: { readings, latestTimestamp },
      }));
    },

    mergeSensorReadings(newReadings: SensorReading[], latestTimestamp: Date | null) {
      update((state) => ({
        ...state,
        sensors: {
          readings: [...state.sensors.readings, ...newReadings],
          latestTimestamp,
        },
      }));
    },

    getSensorTimestamp(): Date | null {
      let timestamp: Date | null = null;
      this.subscribe((state) => {
        timestamp = state.sensors.latestTimestamp;
      })();
      return timestamp;
    },

    // Log methods - updated for timestamp-based tracking

    /**
     * Set log entries for initial load
     */
    setLogEntries(filename: string, entries: LogEntry[], timeRange: TimeRange) {
      const latestEntryTimestamp = entries.length > 0
        ? getEntryTimestamp(entries[entries.length - 1])
        : null;

      update((state) => ({
        ...state,
        logs: {
          ...state.logs,
          [filename]: {
            entries,
            timeRange,
            latestEntryTimestamp,
            loaded: true,
          },
        },
      }));
    },

    /**
     * Append new entries from WebSocket (deduplicates by timestamp)
     */
    appendLogEntries(logFile: LogFile, newEntries: LogEntry[]) {
      const filename = LOG_FILE_TO_FILENAME[logFile];
      if (!filename || newEntries.length === 0) return;

      update((state) => {
        const existing = state.logs[filename];
        if (!existing || !existing.loaded) {
          // Log not loaded yet, ignore WebSocket updates
          return state;
        }

        // Filter out entries we already have (by timestamp)
        const existingLatest = existing.latestEntryTimestamp;
        const filtered = existingLatest
          ? newEntries.filter((e) => getEntryTimestamp(e) > existingLatest)
          : newEntries;

        if (filtered.length === 0) return state;

        const combined = [...existing.entries, ...filtered];
        const newLatest = getEntryTimestamp(filtered[filtered.length - 1]);

        return {
          ...state,
          logs: {
            ...state.logs,
            [filename]: {
              ...existing,
              entries: combined,
              latestEntryTimestamp: newLatest,
            },
          },
        };
      });
    },

    /**
     * Check if a log file is loaded for a specific time range
     */
    isLogLoaded(filename: string, timeRange: TimeRange): boolean {
      let loaded = false;
      this.subscribe((state) => {
        const logCache = state.logs[filename];
        // Loaded if we have data AND the time range matches or is wider
        loaded = logCache?.loaded === true && logCache.timeRange === timeRange;
      })();
      return loaded;
    },

    /**
     * Get cached entries for a log file
     */
    getLogEntries(filename: string): LogEntry[] {
      let entries: LogEntry[] = [];
      this.subscribe((state) => {
        entries = state.logs[filename]?.entries || [];
      })();
      return entries;
    },

    // Clear methods
    clearSensors() {
      update((state) => ({
        ...state,
        sensors: { readings: [], latestTimestamp: null },
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

    clearAllLogs() {
      update((state) => ({
        ...state,
        logs: {},
      }));
    },

    clearAll() {
      update(() => initialState);
    },
  };
}

export const cache = createCache();
