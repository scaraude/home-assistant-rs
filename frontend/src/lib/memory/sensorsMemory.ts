import { writable } from 'svelte/store';
import type { DeviceInfo, SensorReading } from '../api';
import { fetchAggregatedReadings, fetchReadings, fetchSensors } from '../api';
import { dateToUnixSeconds, unixSecondsToDate } from '../utils/time';
import { addRange, coversRange, getMissingRanges, mergeRanges, type Range } from './rangeSet';
import { bulkPut, getAll, put } from './indexedDb';

const RAW_BUCKET = 0;
const RAW_RETENTION_SECONDS = 24 * 3600;
const MAX_BUCKET_POINTS = 1500;
const SERIES_PERSIST_DEBOUNCE_MS = 1000;

interface SeriesState {
  bucketSeconds: number;
  ranges: Range[];
  readings: SensorReading[];
  latestTimestamp: number | null;
  retentionSeconds?: number;
}

interface SensorsMemoryState {
  devices: DeviceInfo[];
  devicesLoaded: boolean;
  seriesByDevice: Record<string, Record<number, SeriesState>>;
  latestByDevice: Record<string, SensorReading | null>;
  hydrated: boolean;
}

type PersistedSensorReading = Omit<SensorReading, 'timestamp'> & { timestamp: number };

type PersistedSeries = {
  key: string;
  device_id: string;
  bucket_seconds: number;
  ranges: Range[];
  readings: PersistedSensorReading[];
  latest_timestamp: number | null;
  updated_at: number;
};

const initialState: SensorsMemoryState = {
  devices: [],
  devicesLoaded: false,
  seriesByDevice: {},
  latestByDevice: {},
  hydrated: false,
};

const { subscribe, update } = writable<SensorsMemoryState>(initialState);

const inflightAggregations = new Set<string>();
const inflightRaw = new Set<string>();
const pendingSeriesPersist = new Map<string, ReturnType<typeof setTimeout>>();

function toPersistedReading(reading: SensorReading): PersistedSensorReading {
  return {
    ...(reading as Omit<SensorReading, 'timestamp'>),
    timestamp: dateToUnixSeconds(reading.timestamp),
  } as PersistedSensorReading;
}

function fromPersistedReading(reading: PersistedSensorReading): SensorReading {
  return {
    ...(reading as Omit<PersistedSensorReading, 'timestamp'>),
    timestamp: unixSecondsToDate(reading.timestamp),
  } as SensorReading;
}

function readingKey(reading: SensorReading): string {
  return `${reading.device_id}:${reading.type}:${dateToUnixSeconds(reading.timestamp)}`;
}

function mergeReadings(existing: SensorReading[], incoming: SensorReading[]): SensorReading[] {
  const map = new Map<string, SensorReading>();
  for (const reading of existing) {
    map.set(readingKey(reading), reading);
  }
  for (const reading of incoming) {
    map.set(readingKey(reading), reading);
  }
  return Array.from(map.values()).sort(
    (a, b) => a.timestamp.getTime() - b.timestamp.getTime(),
  );
}

function computeLatestTimestamp(readings: SensorReading[]): number | null {
  if (readings.length === 0) return null;
  return dateToUnixSeconds(readings[readings.length - 1].timestamp);
}

function rebuildRangesFromReadings(readings: SensorReading[], bucketSeconds: number): Range[] {
  if (readings.length === 0) return [];
  const sorted = [...readings].sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
  if (bucketSeconds <= 0) {
    const start = dateToUnixSeconds(sorted[0].timestamp);
    const end = dateToUnixSeconds(sorted[sorted.length - 1].timestamp);
    return [{ start, end }];
  }

  const ranges: Range[] = [];
  let currentStart = dateToUnixSeconds(sorted[0].timestamp);
  let currentEnd = currentStart;
  const maxGap = bucketSeconds * 2;

  for (let i = 1; i < sorted.length; i += 1) {
    const ts = dateToUnixSeconds(sorted[i].timestamp);
    if (ts - currentEnd <= maxGap) {
      currentEnd = ts;
    } else {
      ranges.push({ start: currentStart, end: currentEnd });
      currentStart = ts;
      currentEnd = ts;
    }
  }
  ranges.push({ start: currentStart, end: currentEnd });
  return mergeRanges(ranges);
}

function trimSeries(series: SeriesState, nowSec: number): SeriesState {
  if (series.bucketSeconds === RAW_BUCKET) {
    const retention = series.retentionSeconds ?? RAW_RETENTION_SECONDS;
    const cutoff = nowSec - retention;
    const filtered = series.readings.filter(
      (reading) => dateToUnixSeconds(reading.timestamp) >= cutoff,
    );
    return {
      ...series,
      readings: filtered,
      ranges: rebuildRangesFromReadings(filtered, RAW_BUCKET),
      latestTimestamp: computeLatestTimestamp(filtered),
    };
  }

  if (series.readings.length <= MAX_BUCKET_POINTS) {
    return series;
  }

  const trimmed = series.readings
    .slice()
    .sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime())
    .slice(-MAX_BUCKET_POINTS);

  return {
    ...series,
    readings: trimmed,
    ranges: rebuildRangesFromReadings(trimmed, series.bucketSeconds),
    latestTimestamp: computeLatestTimestamp(trimmed),
  };
}

function ensureSeries(state: SensorsMemoryState, deviceId: string, bucketSeconds: number): SeriesState {
  const deviceSeries = state.seriesByDevice[deviceId] ?? {};
  const existing = deviceSeries[bucketSeconds];
  if (existing) {
    return existing;
  }

  const next: SeriesState = {
    bucketSeconds,
    ranges: [],
    readings: [],
    latestTimestamp: null,
  };

  state.seriesByDevice = {
    ...state.seriesByDevice,
    [deviceId]: {
      ...deviceSeries,
      [bucketSeconds]: next,
    },
  };

  return next;
}

function scheduleSeriesPersist(deviceId: string, bucketSeconds: number, series: SeriesState) {
  const key = `${deviceId}:${bucketSeconds}`;
  const existingTimer = pendingSeriesPersist.get(key);
  if (existingTimer) {
    clearTimeout(existingTimer);
  }

  const timer = setTimeout(() => {
    pendingSeriesPersist.delete(key);
    const payload: PersistedSeries = {
      key,
      device_id: deviceId,
      bucket_seconds: bucketSeconds,
      ranges: series.ranges,
      readings: series.readings.map(toPersistedReading),
      latest_timestamp: series.latestTimestamp,
      updated_at: Date.now(),
    };
    void put('sensor_series', payload).catch((error) => {
      console.warn('Failed to persist sensor series:', error);
    });
  }, SERIES_PERSIST_DEBOUNCE_MS);

  pendingSeriesPersist.set(key, timer);
}

async function hydrate() {
  try {
    const [devices, series] = await Promise.all([
      getAll<DeviceInfo>('sensor_devices'),
      getAll<PersistedSeries>('sensor_series'),
    ]);

    update((state) => {
      const nextState: SensorsMemoryState = {
        ...state,
        devices,
        devicesLoaded: devices.length > 0,
        seriesByDevice: {},
        latestByDevice: {},
        hydrated: true,
      };

      for (const entry of series) {
        const readings = entry.readings.map(fromPersistedReading);
        const seriesState: SeriesState = {
          bucketSeconds: entry.bucket_seconds,
          ranges: entry.ranges ?? [],
          readings,
          latestTimestamp: entry.latest_timestamp,
        };

        const deviceSeries = nextState.seriesByDevice[entry.device_id] ?? {};
        deviceSeries[entry.bucket_seconds] = seriesState;
        nextState.seriesByDevice[entry.device_id] = deviceSeries;

        if (entry.bucket_seconds === RAW_BUCKET) {
          const latestReading = readings[readings.length - 1] ?? null;
          if (latestReading) {
            nextState.latestByDevice[entry.device_id] = latestReading;
          }
        }
      }

      return nextState;
    });

    pruneAllSeries();
  } catch (error) {
    console.warn('Failed to hydrate sensors memory:', error);
    update((state) => ({ ...state, hydrated: true }));
  }
}

function pruneAllSeries() {
  update((state) => {
    const nowSec = Math.floor(Date.now() / 1000);
    const nextSeriesByDevice: Record<string, Record<number, SeriesState>> = {};
    const nextLatestByDevice: Record<string, SensorReading | null> = { ...state.latestByDevice };

    for (const [deviceId, seriesMap] of Object.entries(state.seriesByDevice)) {
      const nextSeriesMap: Record<number, SeriesState> = {};
      for (const [bucketStr, series] of Object.entries(seriesMap)) {
        const bucketSeconds = Number(bucketStr);
        const trimmed = trimSeries(series, nowSec);
        nextSeriesMap[bucketSeconds] = trimmed;
        if (bucketSeconds === RAW_BUCKET) {
          nextLatestByDevice[deviceId] =
            trimmed.readings[trimmed.readings.length - 1] ?? null;
        }
      }
      nextSeriesByDevice[deviceId] = nextSeriesMap;
    }

    return {
      ...state,
      seriesByDevice: nextSeriesByDevice,
      latestByDevice: nextLatestByDevice,
    };
  });
}

function mergeSeriesReadings(
  deviceId: string,
  bucketSeconds: number,
  incoming: SensorReading[],
  range: Range | null,
  latestTimestamp: number | null,
  retentionSeconds?: number,
) {
  update((state) => {
    const series = ensureSeries(state, deviceId, bucketSeconds);
    const merged = mergeReadings(series.readings, incoming);
    const nowSec = Math.floor(Date.now() / 1000);
    const nextSeries: SeriesState = trimSeries(
      {
        ...series,
        readings: merged,
        latestTimestamp: latestTimestamp ?? computeLatestTimestamp(merged),
        ranges: range ? addRange(series.ranges, range.start, range.end) : series.ranges,
        retentionSeconds: retentionSeconds ?? series.retentionSeconds,
      },
      nowSec,
    );

    const deviceSeries = state.seriesByDevice[deviceId] ?? {};
    const nextDeviceSeries = {
      ...deviceSeries,
      [bucketSeconds]: nextSeries,
    };

    const nextLatestByDevice = { ...state.latestByDevice };
    if (bucketSeconds === RAW_BUCKET) {
      nextLatestByDevice[deviceId] = nextSeries.readings[nextSeries.readings.length - 1] ?? null;
    }

    scheduleSeriesPersist(deviceId, bucketSeconds, nextSeries);

    return {
      ...state,
      seriesByDevice: {
        ...state.seriesByDevice,
        [deviceId]: nextDeviceSeries,
      },
      latestByDevice: nextLatestByDevice,
    };
  });
}

async function ensureDevices(force = false): Promise<DeviceInfo[]> {
  let cached: DeviceInfo[] = [];
  let loaded = false;
  subscribe((state) => {
    cached = state.devices;
    loaded = state.devicesLoaded;
  })();

  if (loaded && !force) {
    return cached;
  }

  const devices = await fetchSensors();
  update((state) => ({
    ...state,
    devices,
    devicesLoaded: true,
  }));

  try {
    await bulkPut('sensor_devices', devices);
  } catch (error) {
    console.warn('Failed to persist sensor devices:', error);
  }

  return devices;
}

async function ensureRecentReadings(deviceId: string, hours: number): Promise<void> {
  const nowSec = Math.floor(Date.now() / 1000);
  const startSec = nowSec - hours * 3600;
  const endSec = nowSec;

  let hasCoverage = false;
  subscribe((state) => {
    const series = state.seriesByDevice[deviceId]?.[RAW_BUCKET];
    hasCoverage = series ? coversRange(series.ranges, startSec, endSec) : false;
  })();

  if (hasCoverage) {
    return;
  }

  const inflightKey = `${deviceId}:${hours}`;
  if (inflightRaw.has(inflightKey)) {
    return;
  }

  inflightRaw.add(inflightKey);
  try {
    const result = await fetchReadings(deviceId, hours);
    mergeSeriesReadings(
      deviceId,
      RAW_BUCKET,
      result.readings,
      { start: startSec, end: endSec },
      result.latestTimestamp ? dateToUnixSeconds(result.latestTimestamp) : null,
      hours * 3600,
    );
  } finally {
    inflightRaw.delete(inflightKey);
  }
}

async function ensureAggregatedSeries(
  deviceId: string,
  startSec: number,
  endSec: number,
  bucketSeconds: number,
): Promise<void> {
  let missing: Range[] = [];
  subscribe((state) => {
    const series = state.seriesByDevice[deviceId]?.[bucketSeconds];
    if (!series) {
      missing = [{ start: startSec, end: endSec }];
      return;
    }
    missing = getMissingRanges(series.ranges, startSec, endSec);
  })();

  if (missing.length === 0) {
    return;
  }

  const requests = missing.map(async (range) => {
    const key = `${deviceId}:${bucketSeconds}:${range.start}:${range.end}`;
    if (inflightAggregations.has(key)) {
      return;
    }
    inflightAggregations.add(key);
    try {
      const result = await fetchAggregatedReadings(
        deviceId,
        new Date(range.start * 1000),
        new Date(range.end * 1000),
        bucketSeconds,
      );
      mergeSeriesReadings(
        deviceId,
        bucketSeconds,
        result.readings,
        range,
        result.latestTimestamp ? dateToUnixSeconds(result.latestTimestamp) : null,
      );
    } finally {
      inflightAggregations.delete(key);
    }
  });

  await Promise.all(requests);
}

function mergeIncomingReading(reading: SensorReading) {
  mergeSeriesReadings(
    reading.device_id,
    RAW_BUCKET,
    [reading],
    {
      start: dateToUnixSeconds(reading.timestamp),
      end: dateToUnixSeconds(reading.timestamp),
    },
    dateToUnixSeconds(reading.timestamp),
  );
}

function applySensorName(deviceId: string, name: string): void {
  update((state) => {
    const updatedDevices = state.devices.map((device) =>
      device.device_id === deviceId ? { ...device, name } : device,
    );
    const updated = updatedDevices.find((device) => device.device_id === deviceId);
    if (updated) {
      void put('sensor_devices', updated).catch((error) => {
        console.warn('Failed to persist sensor name:', error);
      });
    }
    return {
      ...state,
      devices: updatedDevices,
    };
  });
}

function applySensorColor(deviceId: string, color: string): void {
  update((state) => {
    const updatedDevices = state.devices.map((device) =>
      device.device_id === deviceId ? { ...device, color } : device,
    );
    const updated = updatedDevices.find((device) => device.device_id === deviceId);
    if (updated) {
      void put('sensor_devices', updated).catch((error) => {
        console.warn('Failed to persist sensor color:', error);
      });
    }
    return {
      ...state,
      devices: updatedDevices,
    };
  });
}

function getReadings(deviceId: string, startSec: number, endSec: number, bucketSeconds = RAW_BUCKET) {
  let readings: SensorReading[] = [];
  subscribe((state) => {
    const series = state.seriesByDevice[deviceId]?.[bucketSeconds];
    if (!series) {
      readings = [];
      return;
    }
    const startMs = startSec * 1000;
    const endMs = endSec * 1000;
    readings = series.readings.filter((reading) => {
      const ts = reading.timestamp.getTime();
      return ts >= startMs && ts <= endMs;
    });
  })();
  return readings;
}

export const sensorsMemory = {
  subscribe,
  hydrate,
  ensureDevices,
  ensureRecentReadings,
  ensureAggregatedSeries,
  mergeIncomingReading,
  applySensorName,
  applySensorColor,
  getReadings,
};

void hydrate();
