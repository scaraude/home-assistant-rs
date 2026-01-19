import type { DeviceInfo } from './devices';
import { dateToUnixSeconds, parseUnixSeconds, unixSecondsToDate } from '../utils/time';

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
  hours: number = 24
): Promise<{ readings: SensorReading[]; latestTimestamp: Date | null }> {
  const params = new URLSearchParams();
  if (sensorId) {
    params.append('device_id', sensorId);
  }
  params.append('hours', hours.toString());

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
 * Fetch sensor readings since a specific timestamp (delta update)
 * @param sinceTimestamp - Fetch readings after this timestamp
 * @param sensorId - Optional sensor ID filter
 * @returns New readings and updated latest timestamp
 */
export async function fetchReadingsSince(
  sinceTimestamp: Date,
  sensorId?: string
): Promise<{ readings: SensorReading[]; latestTimestamp: Date | null }> {
  const params = new URLSearchParams();
  params.append('since', dateToUnixSeconds(sinceTimestamp).toString());
  if (sensorId) {
    params.append('device_id', sensorId);
  }

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch readings delta: ${response.statusText}`);
  }

  const readings = (await response.json()) as SensorReadingResponse[];
  const latestTimestamp = response.headers.get('X-Latest-Timestamp');

  return {
    readings: readings.map(parseSensorReading),
    latestTimestamp: latestTimestamp ? parseUnixSeconds(parseInt(latestTimestamp, 10)) : null,
  };
}

/**
 * Fetch all sensor data (sensors + their readings)
 * @param hours - Number of hours of history to fetch
 */
export async function fetchAllSensorData(hours: number = 24): Promise<SensorData[]> {
  const [sensors, readingsResult] = await Promise.all([
    fetchSensors(),
    fetchReadings(undefined, hours),
  ]);

  const allReadings = readingsResult.readings;

  return sensors.map((sensor) => {
    const sensorReadings = allReadings
      .filter((r: SensorReading) => r.device_id === sensor.device_id)
      .sort((a: SensorReading, b: SensorReading) => a.timestamp.getTime() - b.timestamp.getTime());

    return {
      name: sensor.name,
      latestReading: sensorReadings[sensorReadings.length - 1] || null,
      history: sensorReadings,
    };
  });
}
