import { writable, get } from "svelte/store";
import {
  fetchDevicePositions,
  updateDevicePosition,
  type DevicePosition,
} from "../api/floor-map";

export interface FloorMapState {
  positions: Record<string, { x: number; y: number }>;
  loading: boolean;
  error: string | null;
  initialized: boolean;
  syncStatus: "idle" | "syncing" | "error";
}

const STORAGE_KEY = "floor-map-positions";
const DEBOUNCE_MS = 300;

// Track pending position updates for debouncing
const pendingUpdates = new Map<string, { x: number; y: number; timeoutId: ReturnType<typeof setTimeout> }>();

/**
 * Load positions from localStorage as fallback
 */
function loadLocalPositions(): Record<string, { x: number; y: number }> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : {};
  } catch {
    return {};
  }
}

/**
 * Save positions to localStorage for offline fallback
 */
function saveLocalPositions(positions: Record<string, { x: number; y: number }>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(positions));
  } catch (e) {
    console.error("[FloorMapStore] Failed to save positions to localStorage:", e);
  }
}

/**
 * Convert API response to positions record
 */
function apiToPositionsRecord(
  positions: DevicePosition[]
): Record<string, { x: number; y: number }> {
  return positions.reduce(
    (acc, pos) => {
      acc[pos.device_id] = { x: pos.x, y: pos.y };
      return acc;
    },
    {} as Record<string, { x: number; y: number }>
  );
}

function createFloorMapStore() {
  const { subscribe, set, update } = writable<FloorMapState>({
    positions: loadLocalPositions(), // Start with localStorage for instant display
    loading: false,
    error: null,
    initialized: false,
    syncStatus: "idle",
  });

  /**
   * Persist a single position update to the backend with debouncing
   */
  async function persistPosition(deviceId: string, x: number, y: number) {
    // Cancel any pending update for this device
    const pending = pendingUpdates.get(deviceId);
    if (pending) {
      clearTimeout(pending.timeoutId);
    }

    // Schedule new debounced update
    const timeoutId = setTimeout(async () => {
      pendingUpdates.delete(deviceId);

      update((state) => ({ ...state, syncStatus: "syncing" }));

      try {
        await updateDevicePosition(deviceId, x, y);
        update((state) => ({ ...state, syncStatus: "idle" }));
      } catch (e) {
        console.error("[FloorMapStore] Failed to persist position:", e);
        update((state) => ({
          ...state,
          syncStatus: "error",
          error: `Failed to save position for device ${deviceId}`,
        }));
        // Position is still saved locally, so user won't lose work
      }
    }, DEBOUNCE_MS);

    pendingUpdates.set(deviceId, { x, y, timeoutId });
  }

  return {
    subscribe,

    /**
     * Initialize the store by loading positions from backend
     * Falls back to localStorage if API fails
     */
    async initialize() {
      const currentState = get({ subscribe });
      if (currentState.initialized) return;

      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const apiPositions = await fetchDevicePositions();
        const positions = apiToPositionsRecord(apiPositions);

        // Merge with localStorage (API takes precedence)
        const localPositions = loadLocalPositions();
        const mergedPositions = { ...localPositions, ...positions };

        // Save merged result to localStorage for offline use
        saveLocalPositions(mergedPositions);

        update((state) => ({
          ...state,
          positions: mergedPositions,
          loading: false,
          initialized: true,
          error: null,
        }));

        console.log("[FloorMapStore] Initialized with", Object.keys(mergedPositions).length, "positions from backend");
      } catch (e) {
        console.error("[FloorMapStore] Failed to load positions from backend, using localStorage:", e);

        // Fall back to localStorage
        const localPositions = loadLocalPositions();
        update((state) => ({
          ...state,
          positions: localPositions,
          loading: false,
          initialized: true,
          error: "Using cached positions (backend unavailable)",
        }));
      }
    },

    /**
     * Update a device position (optimistic update + async persist)
     * This is the main method called during drag operations
     */
    updatePosition(deviceId: string, x: number, y: number) {
      // Optimistic update - immediately reflect in UI
      update((state) => {
        const newPositions = { ...state.positions, [deviceId]: { x, y } };
        // Always save to localStorage for persistence
        saveLocalPositions(newPositions);
        return { ...state, positions: newPositions };
      });

      // Debounced persist to backend
      persistPosition(deviceId, x, y);
    },

    /**
     * Batch update multiple positions (used when receiving WebSocket updates)
     */
    setPositions(positions: Record<string, { x: number; y: number }>) {
      update((state) => {
        const mergedPositions = { ...state.positions, ...positions };
        saveLocalPositions(mergedPositions);
        return { ...state, positions: mergedPositions };
      });
    },

    /**
     * Handle WebSocket event for position update from another client
     */
    handlePositionUpdated(deviceId: string, x: number, y: number) {
      update((state) => {
        // Only update if we don't have a pending update for this device
        // (to avoid overwriting our own optimistic update)
        if (pendingUpdates.has(deviceId)) {
          return state;
        }
        const newPositions = { ...state.positions, [deviceId]: { x, y } };
        saveLocalPositions(newPositions);
        return { ...state, positions: newPositions };
      });
    },

    /**
     * Get position for a device, or undefined if not set
     */
    getPosition(deviceId: string): { x: number; y: number } | undefined {
      return get({ subscribe }).positions[deviceId];
    },

    /**
     * Clear error state
     */
    clearError() {
      update((state) => ({ ...state, error: null }));
    },

    /**
     * Reset store to initial state
     */
    reset() {
      // Cancel all pending updates
      for (const pending of pendingUpdates.values()) {
        clearTimeout(pending.timeoutId);
      }
      pendingUpdates.clear();

      set({
        positions: loadLocalPositions(),
        loading: false,
        error: null,
        initialized: false,
        syncStatus: "idle",
      });
    },
  };
}

export const floorMapStore = createFloorMapStore();
