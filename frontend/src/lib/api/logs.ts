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
 * Time range type for log queries
 */
export type TimeRange = '24h' | '1w' | '1m' | '1y';

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
 * Fetch log file entries since a specific time range (efficient reverse-read)
 * Uses the new /api/logs/since endpoint that reads from end of file
 * @param filename - Log file name (e.g., "system_monitor.log")
 * @param since - Time range like "24h", "1w", "1m", "1y"
 * @returns Log entries from the specified time range
 */
export async function fetchLogsSince(
  filename: string,
  since: TimeRange = '24h'
): Promise<LogEntry[]> {
  const params = new URLSearchParams();
  params.append('file', filename);
  params.append('since', since);

  const response = await fetch(`/api/logs/since?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch logs: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Fetch full history for a specific process/PID combination
 * @param processName - The process name (e.g., "home-automation-rs")
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
