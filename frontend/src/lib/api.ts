// API client for home-assistant-rs backend

export interface SensorReading {
  sensor_id: string;
  temperature: number;
  humidity: number | null;
  battery: number | null;
  link_quality: number | null;
  timestamp: number;
}

export interface SensorData {
  id: string;
  latestReading: SensorReading | null;
  history: SensorReading[];
}

/**
 * Fetch list of all sensor IDs
 */
export async function fetchSensors(): Promise<string[]> {
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
 */
export async function fetchReadings(
  sensorId?: string,
  hours: number = 24
): Promise<SensorReading[]> {
  const params = new URLSearchParams();
  if (sensorId) {
    params.append('sensor_id', sensorId);
  }
  params.append('hours', hours.toString());

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch readings: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Fetch all sensor data (sensors + their readings)
 * @param hours - Number of hours of history to fetch
 */
export async function fetchAllSensorData(hours: number = 24): Promise<SensorData[]> {
  const [sensorIds, allReadings] = await Promise.all([
    fetchSensors(),
    fetchReadings(undefined, hours),
  ]);

  return sensorIds.map((id) => {
    const sensorReadings = allReadings
      .filter((r) => r.sensor_id === id)
      .sort((a, b) => a.timestamp - b.timestamp);

    return {
      id,
      latestReading: sensorReadings[sensorReadings.length - 1] || null,
      history: sensorReadings,
    };
  });
}
