// API client for home-automation-rs backend

export interface SensorReading {
  device_id: string;
  temperature: number;
  humidity: number;  // Now required
  timestamp: number;
  // Note: battery and link_quality are no longer in sensor readings
  // They are now in device_state (see SwitchDevice interface)
}

export interface DeviceInfo {
  device_id: string;
  name: string;
}

export interface SensorData {
  name: string;
  latestReading: SensorReading | null;
  history: SensorReading[];
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
  const [sensors, readingsResult] = await Promise.all([
    fetchSensors(),
    fetchReadings(undefined, hours),
  ]);

  const allReadings = readingsResult.readings;

  return sensors.map((sensor) => {
    const sensorReadings = allReadings
      .filter((r: SensorReading) => r.device_id === sensor.device_id)
      .sort((a: SensorReading, b: SensorReading) => a.timestamp - b.timestamp);

    return {
      name: sensor.name,
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

export interface DeviceState {
  device_id: string;
  battery_level: number | null;
  link_quality: number | null;
  last_seen: number;
}

export interface SwitchDevice {
  id: string;
  name: string;
  state: boolean;
  // Device state fields (from device_state table)
  link_quality: number | null;
  battery_level: number | null;
  last_seen: number | null;
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

/**
 * Open or close Zigbee permit join on Zigbee2MQTT.
 * @param value - true to open the network, false to close it
 * @param time - Optional duration in seconds when opening
 */
export async function setZigbeePermitJoin(value: boolean, time?: number): Promise<void> {
  const payload: { value: boolean; time?: number } = { value };
  if (value && time) {
    payload.time = time;
  }

  const response = await fetch('/api/zigbee/permit_join', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to update Zigbee permit join: ${response.statusText}`);
  }
}

/**
 * Update a device name
 * @param deviceId - Device ID (e.g., "0x7cc6b6fffec90892")
 * @param name - New device name
 */
export async function updateDeviceName(deviceId: string, name: string): Promise<void> {
  const response = await fetch(`/api/devices/${deviceId}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ name }),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to update device name: ${response.statusText}`);
  }
}

/**
 * Fetch device state (battery, link_quality, last_seen)
 * @param deviceId - Device ID (e.g., "0x7cc6b6fffec90892")
 */
export async function fetchDeviceState(deviceId: string): Promise<DeviceState | null> {
  const response = await fetch(`/api/devices/${deviceId}/state`);

  if (response.status === 404) {
    return null; // No state found for this device
  }

  if (!response.ok) {
    throw new Error(`Failed to fetch device state: ${response.statusText}`);
  }

  return response.json();
}

// Automation Rules API Types

export interface AutomationCondition {
  id: string;
  device_id: string;
  field: 'temperature' | 'humidity' | 'battery' | 'link_quality';
  operator: 'equal' | 'not_equal' | 'greater_than' | 'greater_than_or_equal' | 'less_than' | 'less_than_or_equal';
  value: number;
}

export interface AutomationAction {
  id: string;
  device_id: string;
  action: 'on' | 'off' | 'toggle';
}

export interface AutomationRule {
  id: string;
  name: string;
  description: string | null;
  enabled: boolean;
  condition_operator: 'and' | 'or';
  conditions: AutomationCondition[];
  actions: AutomationAction[];
  created_at: string;
  updated_at: string;
  last_triggered_at: string | null;
  trigger_count: number;
}

export interface CreateAutomationRuleRequest {
  name: string;
  description?: string;
  enabled?: boolean;
  condition_operator: 'and' | 'or';
  conditions: Array<{
    device_id: string;
    field: string;
    operator: string;
    value: number;
  }>;
  actions: Array<{
    device_id: string;
    action: string;
  }>;
}

export interface UpdateAutomationRuleRequest {
  name?: string;
  description?: string;
  enabled?: boolean;
  condition_operator?: 'and' | 'or';
  conditions?: Array<{
    device_id: string;
    field: string;
    operator: string;
    value: number;
  }>;
  actions?: Array<{
    device_id: string;
    action: string;
  }>;
}

export interface AutomationExecutionLog {
  id: string;
  rule_id: string;
  rule_name: string;
  success: boolean;
  error_message: string | null;
  executed_at: string;
}

// Automation Rules API Functions

/**
 * Fetch all automation rules
 */
export async function fetchAutomationRules(): Promise<AutomationRule[]> {
  const response = await fetch('/api/automation/rules');
  if (!response.ok) {
    throw new Error(`Failed to fetch automation rules: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Fetch a single automation rule by ID
 * @param id - Rule ID
 */
export async function fetchAutomationRule(id: string): Promise<AutomationRule> {
  const response = await fetch(`/api/automation/rules/${id}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch automation rule: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Create a new automation rule
 * @param rule - Rule creation request
 */
export async function createAutomationRule(rule: CreateAutomationRuleRequest): Promise<AutomationRule> {
  const response = await fetch('/api/automation/rules', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(rule),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to create automation rule: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Update an existing automation rule
 * @param id - Rule ID
 * @param updates - Partial rule updates
 */
export async function updateAutomationRule(
  id: string,
  updates: UpdateAutomationRuleRequest
): Promise<AutomationRule> {
  const response = await fetch(`/api/automation/rules/${id}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(updates),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to update automation rule: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Delete an automation rule
 * @param id - Rule ID
 */
export async function deleteAutomationRule(id: string): Promise<void> {
  const response = await fetch(`/api/automation/rules/${id}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to delete automation rule: ${response.statusText}`);
  }
}

/**
 * Fetch automation execution logs
 * @param limit - Maximum number of log entries (default: 100)
 */
export async function fetchAutomationLogs(limit: number = 100): Promise<AutomationExecutionLog[]> {
  const params = new URLSearchParams();
  params.append('limit', limit.toString());

  const response = await fetch(`/api/automation/logs?${params.toString()}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch automation logs: ${response.statusText}`);
  }
  return response.json();
}
