import type { DeviceInfo } from './devices';
import { dateToUnixSeconds, parseUnixSeconds, unixSecondsToDate } from '../utils/time';

/// Target number of points per series when auto-bucketing a time range.
const TARGET_POINTS = 500;

export interface TempHumiditySensorReading {
  type: 'temp_humidity';
  device_id: string;
  temperature: number;
  humidity: number;
  timestamp: Date;
}

export interface PresenceSensorReading {
  type: 'presence';
  device_id: string;
  occupied: boolean;
  illumination?: string;
  timestamp: Date;
}

export interface EnergySensorReading {
  type: 'energy_meter';
  device_id: string;
  power: number;              // Watts (primary metric)
  energy: number;             // kWh consumed
  produced_energy: number;    // kWh produced
  voltage: number;            // Volts
  current: number;            // Amperes
  ac_frequency: number;       // Hertz
  power_factor: number;       // 0-1
  timestamp: Date;
}

export type SensorReading = TempHumiditySensorReading | PresenceSensorReading | EnergySensorReading;

export interface SensorData {
  name: string;
  latestReading: SensorReading | null;
  history: SensorReading[];
}

export type SensorReadingResponse =
  | (Omit<TempHumiditySensorReading, 'timestamp'> & { timestamp: number })
  | (Omit<PresenceSensorReading, 'timestamp'> & { timestamp: number })
  | (Omit<EnergySensorReading, 'timestamp'> & { timestamp: number });

export function parseSensorReading(reading: SensorReadingResponse): SensorReading {
  return {
    ...reading,
    timestamp: unixSecondsToDate(reading.timestamp),
  };
}

/**
 * Fetch list of all sensors with their device info (ID + name)
 */
export async function fetchSensors(): Promise<DeviceInfo[]> {
  const response = await fetch('/api/sensors');
  if (!response.ok) {
    throw new Error(`Failed to fetch sensors: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Fetch sensor readings with optional filters
 * @param sensorId - Optional sensor ID filter
 * @param hours - Number of hours to fetch (default: 24)
 * @returns Readings and latest timestamp from X-Latest-Timestamp header
 */
export async function fetchReadings(
  sensorId?: string,
  hours: number = 24,
  bucketSeconds?: number
): Promise<{ readings: SensorReading[]; latestTimestamp: Date | null }> {
  const params = new URLSearchParams();
  if (sensorId) {
    params.append('device_id', sensorId);
  }
  params.append('hours', hours.toString());

  // Always server-side aggregate so the payload stays bounded (~500 points)
  // instead of pulling every raw sample — a high-frequency energy meter can
  // otherwise return tens of thousands of rows for a 24h window. Callers that
  // truly need raw data can pass bucketSeconds = 1.
  const bucket =
    bucketSeconds ?? Math.max(60, Math.floor((hours * 3600) / TARGET_POINTS));
  params.append('bucket', bucket.toString());

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch readings: ${response.statusText}`);
  }

  const readings = (await response.json()) as SensorReadingResponse[];
  const latestTimestamp = response.headers.get('X-Latest-Timestamp');

  return {
    readings: readings.map(parseSensorReading),
    latestTimestamp: latestTimestamp ? parseUnixSeconds(parseInt(latestTimestamp, 10)) : null,
  };
}

/**
 * Fetch latest reading(s) only.
 * @param sensorId - Optional sensor ID filter
 */
export async function fetchLatestReadings(sensorId?: string): Promise<SensorReading[]> {
  const params = new URLSearchParams();
  params.append('latest', '1');
  if (sensorId) {
    params.append('device_id', sensorId);
  }

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch latest readings: ${response.statusText}`);
  }

  const readings = (await response.json()) as SensorReadingResponse[];
  return readings.map(parseSensorReading);
}

/**
 * Fetch aggregated sensor readings for a specific device and time range
 * @param sensorId - Sensor device ID (required)
 * @param start - Range start time
 * @param end - Range end time
 * @param bucketSeconds - Bucket size in seconds
 */
export async function fetchAggregatedReadings(
  sensorId: string,
  start: Date,
  end: Date,
  bucketSeconds: number,
): Promise<{ readings: SensorReading[]; latestTimestamp: Date | null }> {
  const params = new URLSearchParams();
  params.append('device_id', sensorId);
  params.append('start', dateToUnixSeconds(start).toString());
  params.append('end', dateToUnixSeconds(end).toString());
  params.append('bucket', Math.max(1, Math.floor(bucketSeconds)).toString());

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch aggregated readings: ${response.statusText}`);
  }

  const readings = (await response.json()) as SensorReadingResponse[];
  const latestTimestamp = response.headers.get('X-Latest-Timestamp');

  return {
    readings: readings.map(parseSensorReading),
    latestTimestamp: latestTimestamp ? parseUnixSeconds(parseInt(latestTimestamp, 10)) : null,
  };
}
