import { writable } from 'svelte/store';

export interface Device {
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
}

export interface NetworkEdge {
  source_id: string;
  target_id: string;
  link_quality: number | null;
}

export interface NetworkTopology {
  devices: Device[];
  edges: NetworkEdge[];
}

export interface NetworkState {
  topology: NetworkTopology | null;
  positions: Record<string, { x: number; y: number }>;
  loading: boolean;
  error: string | null;
}

const STORAGE_KEY = 'network-topology-positions';

function loadPositions(): Record<string, { x: number; y: number }> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : {};
  } catch {
    return {};
  }
}

function savePositions(positions: Record<string, { x: number; y: number }>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(positions));
  } catch (e) {
    console.error('Failed to save node positions:', e);
  }
}

function createNetworkTopologyStore() {
  const { subscribe, set, update } = writable<NetworkState>({
    topology: null,
    positions: loadPositions(),
    loading: false,
    error: null,
  });

  return {
    subscribe,
    setTopology: (topology: NetworkTopology) => {
      console.log('[Store] setTopology called with:', topology);
      update((state) => {
        const newState = {
          ...state,
          topology,
          loading: false,
          error: null,
        };
        console.log('[Store] New state after setTopology:', newState);
        return newState;
      });
    },
    setLoading: (loading: boolean) => {
      console.log('[Store] setLoading called with:', loading);
      update((state) => ({ ...state, loading }));
    },
    setError: (error: string) => {
      console.log('[Store] setError called with:', error);
      update((state) => ({ ...state, error, loading: false }));
    },
    updateNodePosition: (nodeId: string, x: number, y: number) => {
      update((state) => {
        const newPositions = { ...state.positions, [nodeId]: { x, y } };
        savePositions(newPositions);
        return { ...state, positions: newPositions };
      });
    },
    reset: () => {
      set({
        topology: null,
        positions: loadPositions(),
        loading: false,
        error: null,
      });
    },
  };
}

export const networkTopologyStore = createNetworkTopologyStore();
