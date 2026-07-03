import { dateToUnixSeconds, unixSecondsToDate } from '../utils/time';

/**
 * Cumulative-energy summary for a period.
 *
 * The meter's `energy`/`produced_energy` are lifetime counters, so the server
 * returns per-bucket **deltas** (consumption/production during the bucket), the
 * period totals, and the peak instantaneous power. The totals always equal the
 * sum of the buckets, so the histogram and the headline figure agree.
 */
export interface EnergyBucket {
  /** Bucket start. */
  timestamp: Date;
  /** kWh consumed during the bucket. */
  consumed: number;
  /** kWh produced during the bucket. */
  produced: number;
}

export interface EnergySummary {
  start: Date;
  end: Date;
  bucketSeconds: number;
  totalConsumed: number;
  totalProduced: number;
  /** Highest single power sample over the period (W). */
  peakPower: number;
  /** When the peak power occurred, or null if there is no data. */
  peakPowerTimestamp: Date | null;
  buckets: EnergyBucket[];
}

interface EnergySummaryResponse {
  start: number;
  end: number;
  bucket_seconds: number;
  total_consumed: number;
  total_produced: number;
  peak_power: number;
  peak_power_timestamp: number | null;
  buckets: { timestamp: number; consumed: number; produced: number }[];
}

export interface FetchEnergySummaryOptions {
  /** Restrict to one meter; omit to aggregate across every energy meter. */
  deviceId?: string;
  start: Date;
  end: Date;
  /** Bucket size in seconds (e.g. 3600 for hourly, 86400 for daily). */
  bucketSeconds: number;
}

/**
 * Fetch a cumulative-energy summary from `/api/energy/summary`.
 */
export async function fetchEnergySummary(
  opts: FetchEnergySummaryOptions,
): Promise<EnergySummary> {
  const params = new URLSearchParams();
  if (opts.deviceId) params.append('device_id', opts.deviceId);
  params.append('start', dateToUnixSeconds(opts.start).toString());
  params.append('end', dateToUnixSeconds(opts.end).toString());
  params.append('bucket', Math.max(1, Math.floor(opts.bucketSeconds)).toString());

  const response = await fetch(`/api/energy/summary?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch energy summary: ${response.statusText}`);
  }

  const data = (await response.json()) as EnergySummaryResponse;
  return {
    start: unixSecondsToDate(data.start),
    end: unixSecondsToDate(data.end),
    bucketSeconds: data.bucket_seconds,
    totalConsumed: data.total_consumed,
    totalProduced: data.total_produced,
    peakPower: data.peak_power,
    peakPowerTimestamp:
      data.peak_power_timestamp != null
        ? unixSecondsToDate(data.peak_power_timestamp)
        : null,
    buckets: data.buckets.map((b) => ({
      timestamp: unixSecondsToDate(b.timestamp),
      consumed: b.consumed,
      produced: b.produced,
    })),
  };
}
