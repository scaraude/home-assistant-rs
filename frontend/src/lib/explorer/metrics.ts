import type { DeviceInfo } from '../api/devices';
import type { SensorReading } from '../api';
import type { SensorType } from '../types/sensors';

/**
 * A "metric" is one plottable channel. A single temp/humidity device exposes
 * two of them (temperature + humidity), which is why the explorer keys series
 * by (deviceId, metric) rather than by device alone.
 */
export type ExplorerMetric = 'temperature' | 'humidity' | 'power' | 'presence';

export type MetricKind = 'line' | 'band';

export interface CategoryDef {
  /** metric id, also used as the left-rail tab id */
  id: ExplorerMetric;
  label: string;
  /** display unit; null for presence (no value axis — rendered as bands) */
  unit: string | null;
  sensorType: SensorType;
  kind: MetricKind;
  /** inline SVG path drawn in a 24x24 viewBox */
  iconPath: string;
}

export const CATEGORIES: CategoryDef[] = [
  {
    id: 'temperature',
    label: 'Température',
    unit: '°C',
    sensorType: 'temp_humidity',
    kind: 'line',
    iconPath:
      'M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0zM12 3.5a.5.5 0 0 1 1 0V12h-1z',
  },
  {
    id: 'humidity',
    label: 'Humidité',
    unit: '%',
    sensorType: 'temp_humidity',
    kind: 'line',
    iconPath: 'M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z',
  },
  {
    id: 'presence',
    label: 'Présence',
    unit: null,
    sensorType: 'presence',
    kind: 'band',
    iconPath:
      'M12 2a2 2 0 1 1 0 4 2 2 0 0 1 0-4zm-1.5 6h3l3 6-2 1-2-3v10h-2V13l-2 3-2-1z',
  },
  {
    id: 'power',
    label: 'Énergie',
    unit: 'W',
    sensorType: 'energy_meter',
    kind: 'line',
    iconPath: 'M13 2L3 14h7l-1 8 10-12h-7z',
  },
];

export const CATEGORY_BY_ID: Record<ExplorerMetric, CategoryDef> = Object.fromEntries(
  CATEGORIES.map((c) => [c.id, c]),
) as Record<ExplorerMetric, CategoryDef>;

export function seriesKey(deviceId: string, metric: ExplorerMetric): string {
  return `${deviceId}:${metric}`;
}

/** Does this device expose the given metric? */
export function deviceHasMetric(device: DeviceInfo, metric: ExplorerMetric): boolean {
  const { sensorType } = CATEGORY_BY_ID[metric];
  return device.capabilities.some(
    (cap) => cap.type === 'sensor' && cap.sensor_type === sensorType,
  );
}

/** Devices offering the metric of a given category. */
export function devicesForCategory(devices: DeviceInfo[], metric: ExplorerMetric): DeviceInfo[] {
  return devices.filter((d) => deviceHasMetric(d, metric));
}

/**
 * Numeric value of a reading for a metric, or null if the reading isn't of the
 * matching type. Presence returns 0/1 (used only for band detection).
 */
export function metricValue(reading: SensorReading, metric: ExplorerMetric): number | null {
  switch (metric) {
    case 'temperature':
      return reading.type === 'temp_humidity' ? reading.temperature : null;
    case 'humidity':
      return reading.type === 'temp_humidity' ? reading.humidity : null;
    case 'power':
      return reading.type === 'energy_meter' ? reading.power : null;
    case 'presence':
      return reading.type === 'presence' ? (reading.occupied ? 1 : 0) : null;
  }
}

export function formatMetricValue(value: number, metric: ExplorerMetric): string {
  const unit = CATEGORY_BY_ID[metric].unit;
  if (metric === 'presence') return value >= 0.5 ? 'Occupé' : 'Libre';
  const digits = metric === 'power' ? 0 : 1;
  return `${value.toFixed(digits)}${unit ? ` ${unit}` : ''}`;
}
