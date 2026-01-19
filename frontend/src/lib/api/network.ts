export interface NetworkTopology {
  devices: Array<{
    id: string;
    mqtt_topic: string;
    ieee_addr: string;
    name: string;
    capability: {
      type: 'sensor' | 'commander' | 'coordinator';
      sensor_type?: 'temp_humidity' | 'presence';
      commander_type?: 'switch';
    };
    power_source: 'battery' | 'plugged';
    added_at: number;
    is_bridge: boolean;
    parent_device_id: string | null;
  }>;
  edges: Array<{
    source_id: string;
    target_id: string;
    link_quality: number | null;
  }>;
}

/**
 * Fetch network topology (devices + edges)
 */
export async function fetchNetworkTopology(): Promise<NetworkTopology> {
  const response = await fetch('/api/network/topology');
  if (!response.ok) {
    throw new Error(`Failed to fetch network topology: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Request a network map refresh from zigbee2mqtt
 */
export async function refreshNetworkMap(): Promise<void> {
  const response = await fetch('/api/network/refresh', {
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error(`Failed to refresh network map: ${response.statusText}`);
  }
}
