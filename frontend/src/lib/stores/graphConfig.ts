import { writable } from 'svelte/store';

export interface SensorUIConfig {
  deviceId: string;
  color: string;
  visible: boolean;
}

export interface GraphState {
  metric: 'temperature' | 'humidity' | 'both';
  timeRange: '24h' | '1w' | '1m' | '1y';
  sensors: SensorUIConfig[];
}

// Perceptually distinct, colorblind-safe palette
export const RECOMMENDED_COLORS = [
  '#1f77b4', // Blue
  '#ff7f0e', // Orange
  '#2ca02c', // Green
  '#d62728', // Red
  '#9467bd', // Purple
  '#8c564b', // Brown
  '#e377c2', // Pink
  '#7f7f7f', // Gray
  '#bcbd22', // Olive
  '#17becf', // Cyan
  '#003f5c', // Navy
  '#ffa600', // Amber
];

// Map time range to hours for API calls
export const TIME_RANGE_HOURS: Record<GraphState['timeRange'], number> = {
  '24h': 24,
  '1w': 168,
  '1m': 720,
  '1y': 8760,
};

const STORAGE_KEY = 'homeAutomation:graphConfig';

// Initial state
const initialState: GraphState = {
  metric: 'temperature',
  timeRange: '24h',
  sensors: [],
};

// Load persisted preferences from localStorage
function loadPersistedState(): Partial<GraphState> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      return {
        metric: parsed.metric || 'temperature',
        timeRange: parsed.timeRange || '24h',
        sensors: parsed.sensors || [],
      };
    }
  } catch (e) {
    console.warn('Failed to load graph config from localStorage:', e);
  }
  return {};
}

function createGraphConfig() {
  const persisted = loadPersistedState();
  const { subscribe, set, update } = writable<GraphState>({
    ...initialState,
    ...persisted,
  });

  // Persist to localStorage on changes (debounced)
  let saveTimeout: ReturnType<typeof setTimeout>;
  function persist(state: GraphState) {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify({
          metric: state.metric,
          timeRange: state.timeRange,
          sensors: state.sensors.map(s => ({
            deviceId: s.deviceId,
            color: s.color,
            // Don't persist visible state - default to all hidden on load
          })),
        }));
      } catch (e) {
        console.warn('Failed to persist graph config:', e);
      }
    }, 500);
  }

  return {
    subscribe,
    set,
    update,

    // Reset sensors list (call when switching views)
    reset: () => {
      update(state => ({
        ...state,
        sensors: [],
      }));
    },

    // Initialize sensors from device list
    // Preserves existing visibility state for sensors that are already in the list
    initializeSensors: (devices: Array<{ deviceId: string; color?: string | null }>) => {
      update(state => {
        // Build a map of existing sensors to preserve their visibility state
        const existingSensors = new Map(
          state.sensors.map(s => [s.deviceId, s])
        );

        const sensors: SensorUIConfig[] = devices.map((device, idx) => {
          const existing = existingSensors.get(device.deviceId);
          const deviceColor = device.color?.trim() ? device.color : undefined;
          return {
            deviceId: device.deviceId,
            color: deviceColor || existing?.color || RECOMMENDED_COLORS[idx % RECOMMENDED_COLORS.length],
            // Preserve visibility if sensor exists, otherwise default to hidden
            visible: existing?.visible ?? false,
          };
        });

        const newState = { ...state, sensors };
        persist(newState);
        return newState;
      });
    },

    // Toggle single sensor visibility
    toggleSensor: (deviceId: string) => {
      update(state => {
        const newState = {
          ...state,
          sensors: state.sensors.map(s =>
            s.deviceId === deviceId ? { ...s, visible: !s.visible } : s
          ),
        };
        return newState; // Don't persist visibility
      });
    },

    // Show only one sensor (isolate)
    isolateSensor: (deviceId: string) => {
      update(state => {
        const newState = {
          ...state,
          sensors: state.sensors.map(s => ({
            ...s,
            visible: s.deviceId === deviceId,
          })),
        };
        return newState; // Don't persist visibility
      });
    },

    // Show all sensors
    showAll: () => {
      update(state => {
        const newState = {
          ...state,
          sensors: state.sensors.map(s => ({ ...s, visible: true })),
        };
        return newState; // Don't persist visibility
      });
    },

    // Hide all sensors
    hideAll: () => {
      update(state => {
        const newState = {
          ...state,
          sensors: state.sensors.map(s => ({ ...s, visible: false })),
        };
        return newState; // Don't persist visibility
      });
    },

    // Update sensor color
    setSensorColor: (deviceId: string, color: string) => {
      update(state => {
        const newState = {
          ...state,
          sensors: state.sensors.map(s =>
            s.deviceId === deviceId ? { ...s, color } : s
          ),
        };
        persist(newState);
        return newState;
      });
    },

    // Change metric type
    setMetric: (metric: GraphState['metric']) => {
      update(state => {
        const newState = { ...state, metric };
        persist(newState);
        return newState;
      });
    },

    // Change time range
    setTimeRange: (timeRange: GraphState['timeRange']) => {
      update(state => {
        const newState = { ...state, timeRange };
        persist(newState);
        return newState;
      });
    },
  };
}

export const graphConfig = createGraphConfig();
