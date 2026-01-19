export interface DeviceInfo {
  device_id: string;
  name: string;
  capabilities: DeviceCapability[];
  available_fields: string[];
}

export type DeviceCapability =
  | { type: 'sensor'; sensor_type: 'temp_humidity' | 'presence' | 'energy_meter' }
  | { type: 'commander'; commander_type: 'switch' }
  | { type: 'router'; turbo_mode: boolean }
  | { type: 'coordinator' };

export interface DeviceState {
  device_id: string;
  battery_level: number | null;
  link_quality: number | null;
  last_seen: number;
  turbo_mode: boolean | null;
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
 * Update a device option (e.g. turbo_mode)
 * @param deviceId - Device ID (e.g., "0x7cc6b6fffec90892")
 * @param option - Option name (e.g., "turbo_mode")
 * @param value - Option value
 */
export async function setDeviceOption(
  deviceId: string,
  option: string,
  value: unknown,
): Promise<void> {
  const response = await fetch(`/api/devices/${deviceId}/set`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ [option]: value }),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(errorData.error || `Failed to set device option: ${response.statusText}`);
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

/**
 * Fetch device state for all devices (battery, link_quality, last_seen)
 */
export async function fetchDeviceStates(): Promise<DeviceState[]> {
  const response = await fetch('/api/devices/state');

  if (!response.ok) {
    throw new Error(`Failed to fetch device states: ${response.statusText}`);
  }

  return response.json();
}
