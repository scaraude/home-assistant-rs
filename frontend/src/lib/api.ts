// API client for home-assistant-rs backend

export interface SensorReading {
  device_id: string;
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
 * @returns Readings and latest timestamp from X-Latest-Timestamp header
 */
export async function fetchReadings(
  sensorId?: string,
  hours: number = 24
): Promise<{ readings: SensorReading[]; latestTimestamp: number | null }> {
  const params = new URLSearchParams();
  if (sensorId) {
    params.append('device_id', sensorId);
  }
  params.append('hours', hours.toString());

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch readings: ${response.statusText}`);
  }

  const readings = await response.json();
  const latestTimestamp = response.headers.get('X-Latest-Timestamp');

  return {
    readings,
    latestTimestamp: latestTimestamp ? parseInt(latestTimestamp, 10) : null,
  };
}

/**
 * Fetch sensor readings since a specific timestamp (delta update)
 * @param sinceTimestamp - Unix timestamp to fetch readings after
 * @param sensorId - Optional sensor ID filter
 * @returns New readings and updated latest timestamp
 */
export async function fetchReadingsSince(
  sinceTimestamp: number,
  sensorId?: string
): Promise<{ readings: SensorReading[]; latestTimestamp: number | null }> {
  const params = new URLSearchParams();
  params.append('since', sinceTimestamp.toString());
  if (sensorId) {
    params.append('device_id', sensorId);
  }

  const response = await fetch(`/api/readings?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch readings delta: ${response.statusText}`);
  }

  const readings = await response.json();
  const latestTimestamp = response.headers.get('X-Latest-Timestamp');

  return {
    readings,
    latestTimestamp: latestTimestamp ? parseInt(latestTimestamp, 10) : null,
  };
}

/**
 * Fetch all sensor data (sensors + their readings)
 * @param hours - Number of hours of history to fetch
 */
export async function fetchAllSensorData(hours: number = 24): Promise<SensorData[]> {
  const [sensorIds, readingsResult] = await Promise.all([
    fetchSensors(),
    fetchReadings(undefined, hours),
  ]);

  const allReadings = readingsResult.readings;

  return sensorIds.map((id) => {
    const sensorReadings = allReadings
      .filter((r: SensorReading) => r.device_id === id)
      .sort((a: SensorReading, b: SensorReading) => a.timestamp - b.timestamp);

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

// Switch API Types

export interface SwitchDevice {
  id: string;
  name: string;
  state: boolean;
  link_quality: number | null;
  last_updated: number;
}

export interface SwitchCommand {
  device_id: string;
  state: boolean;
}

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
 * @returns Log entries and total lines from X-Total-Lines header
 */
export async function fetchLogView(
  filename: string,
  maxLines: number = 1000
): Promise<{ entries: LogEntry[]; totalLines: number }> {
  const params = new URLSearchParams();
  params.append('file', filename);
  params.append('lines', maxLines.toString());

  const response = await fetch(`/api/logs/view?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch log view: ${response.statusText}`);
  }

  const entries = await response.json();
  const totalLines = response.headers.get('X-Total-Lines');

  return {
    entries,
    totalLines: totalLines ? parseInt(totalLines, 10) : 0,
  };
}

/**
 * Fetch log file content since a specific line number (delta update)
 * @param filename - Log file name
 * @param sinceLine - Line number to fetch entries after
 * @returns New log entries and updated total lines
 */
export async function fetchLogViewSince(
  filename: string,
  sinceLine: number
): Promise<{ entries: LogEntry[]; totalLines: number }> {
  const params = new URLSearchParams();
  params.append('file', filename);
  params.append('since_line', sinceLine.toString());

  const response = await fetch(`/api/logs/view?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch log view delta: ${response.statusText}`);
  }

  const entries = await response.json();
  const totalLines = response.headers.get('X-Total-Lines');

  return {
    entries,
    totalLines: totalLines ? parseInt(totalLines, 10) : 0,
  };
}

/**
 * Fetch full history for a specific process/PID combination
 * @param processName - The process name (e.g., "home-assistant-rs")
 * @param pid - The process ID
 * @param maxLines - Maximum number of lines to fetch (default: 10000)
 * @returns Process history entries
 */
export async function fetchProcessHistory(
  processName: string,
  pid: string,
  maxLines: number = 10000
): Promise<ProcessMonitorEntry[]> {
  const params = new URLSearchParams();
  params.append('process', processName);
  params.append('pid', pid);
  params.append('lines', maxLines.toString());

  const response = await fetch(`/api/logs/process?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch process history: ${response.statusText}`);
  }

  const entries = await response.json();
  return entries;
}

// Switch API Functions

/**
 * Fetch list of available switches
 */
export async function fetchSwitches(): Promise<SwitchDevice[]> {
  const response = await fetch('/api/devices/switches');
  if (!response.ok) {
    throw new Error(`Failed to fetch switches: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Execute a command on a switch device
 * @param deviceId - Device ID (e.g., "0x7cc6b6fffec90892")
 * @param state - Desired state (true = ON, false = OFF)
 */
export async function executeCommand(deviceId: string, state: boolean): Promise<void> {
  const command: SwitchCommand = {
    device_id: deviceId,
    state,
  };

  const response = await fetch('/api/commands/execute', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(command),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to execute command: ${response.statusText}`);
  }
}
