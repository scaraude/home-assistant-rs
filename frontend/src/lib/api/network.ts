import type { NetworkDevice } from "../types/devices";
import { unixSecondsToDate } from "../utils/time";

export interface NetworkTopology {
  devices: NetworkDevice[];
  edges: Array<{
    source_id: string;
    target_id: string;
    link_quality: number | null;
  }>;
}

type NetworkDeviceResponse = Omit<NetworkDevice, "added_at"> & { added_at: number };
type NetworkTopologyResponse = Omit<NetworkTopology, "devices"> & {
  devices: NetworkDeviceResponse[];
};

/**
 * Fetch network topology (devices + edges)
 */
export async function fetchNetworkTopology(): Promise<NetworkTopology> {
  const response = await fetch('/api/network/topology');
  if (!response.ok) {
    throw new Error(`Failed to fetch network topology: ${response.statusText}`);
  }
  const topology = (await response.json()) as NetworkTopologyResponse;
  return {
    ...topology,
    devices: topology.devices.map((device) => ({
      ...device,
      added_at: unixSecondsToDate(device.added_at),
    })),
  };
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
