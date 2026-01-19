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
