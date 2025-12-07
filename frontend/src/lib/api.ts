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

// Log API Types

export interface LogFileInfo {
  name: string;
  display_name: string;
}

export interface SystemMonitorEntry {
  timestamp: string;
  cpu_usage: number;
  ram_used: number;
  ram_total: number;
  ram_usage: number;
  cpu_temp: number;
}

export interface ProcessMonitorEntry {
  timestamp: string;
  process: string;
  pid: string;
  cpu: number;
  ram: number;
  status: string;
}

export interface TopConsumerEntry {
  timestamp: string;
  rank: number;
  process: string;
  pid: string;
  cpu: number;
  ram: number;
}

export type LogEntry = SystemMonitorEntry | ProcessMonitorEntry | TopConsumerEntry;

/**
 * Fetch list of available log files
 */
export async function fetchLogFiles(): Promise<LogFileInfo[]> {
  const response = await fetch('/api/logs/list');
  if (!response.ok) {
    throw new Error(`Failed to fetch log files: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Fetch log file content
 * @param filename - Log file name (e.g., "system_monitor.log")
 * @param maxLines - Maximum number of lines to fetch (default: 1000)
 */
export async function fetchLogView(
  filename: string,
  maxLines: number = 1000
): Promise<LogEntry[]> {
  const params = new URLSearchParams();
  params.append('file', filename);
  params.append('lines', maxLines.toString());

  const response = await fetch(`/api/logs/view?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch log view: ${response.statusText}`);
  }
  return response.json();
}
