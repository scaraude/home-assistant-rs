import { writable } from 'svelte/store';
import type { DeviceState } from '../types/devices';
import { fetchDeviceState, fetchDeviceStates } from '../api';
import { unixSecondsToDate, dateToUnixSeconds } from '../utils/time';
import { bulkPut, getAll, put } from './indexedDb';

interface DeviceStateMemoryState {
  byId: Record<string, DeviceState>;
  loaded: boolean;
  hydrated: boolean;
}

type PersistedDeviceState = Omit<DeviceState, 'last_seen'> & { last_seen: number };

const initialState: DeviceStateMemoryState = {
  byId: {},
  loaded: false,
  hydrated: false,
};

const { subscribe, update } = writable<DeviceStateMemoryState>(initialState);

function toPersisted(state: DeviceState): PersistedDeviceState {
  return {
    ...state,
    last_seen: dateToUnixSeconds(state.last_seen),
  };
}

function fromPersisted(state: PersistedDeviceState): DeviceState {
  return {
    ...state,
    last_seen: unixSecondsToDate(state.last_seen),
  };
}

async function hydrate() {
  try {
    const states = await getAll<PersistedDeviceState>('device_states');
    const parsed = states.map(fromPersisted);
    const byId = parsed.reduce<Record<string, DeviceState>>((acc, state) => {
      acc[state.device_id] = state;
      return acc;
    }, {});
    update((prev) => ({
      ...prev,
      byId,
      loaded: parsed.length > 0,
      hydrated: true,
    }));
  } catch (error) {
    console.warn('Failed to hydrate device states:', error);
    update((prev) => ({ ...prev, hydrated: true }));
  }
}

function mergeState(existing: DeviceState | undefined, updates: Partial<DeviceState>): DeviceState {
  return {
    device_id: updates.device_id ?? existing?.device_id ?? '',
    battery_level: updates.battery_level ?? existing?.battery_level ?? null,
    link_quality: updates.link_quality ?? existing?.link_quality ?? null,
    turbo_mode: updates.turbo_mode ?? existing?.turbo_mode ?? null,
    last_seen: updates.last_seen ?? existing?.last_seen ?? new Date(),
  };
}

function updateDeviceState(deviceId: string, updates: Partial<DeviceState>) {
  update((state) => {
    const existing = state.byId[deviceId];
    const nextState = mergeState(existing, { ...updates, device_id: deviceId });

    void put('device_states', toPersisted(nextState)).catch((error) => {
      console.warn('Failed to persist device state:', error);
    });

    return {
      ...state,
      byId: {
        ...state.byId,
        [deviceId]: nextState,
      },
    };
  });
}

async function ensureDeviceState(deviceId: string, force = false): Promise<DeviceState | null> {
  let cached: DeviceState | undefined;
  subscribe((state) => {
    cached = state.byId[deviceId];
  })();

  if (cached && !force) {
    return cached;
  }

  const fetched = await fetchDeviceState(deviceId);
  if (fetched) {
    updateDeviceState(deviceId, fetched);
  }
  return fetched ?? null;
}

async function ensureDeviceStates(force = false): Promise<DeviceState[]> {
  let loaded = false;
  subscribe((state) => {
    loaded = state.loaded;
  })();

  if (loaded && !force) {
    let cached: DeviceState[] = [];
    subscribe((state) => {
      cached = Object.values(state.byId);
    })();
    return cached;
  }

  const states = await fetchDeviceStates();
  update((prev) => {
    const byId = { ...prev.byId };
    for (const state of states) {
      byId[state.device_id] = state;
    }
    return {
      ...prev,
      byId,
      loaded: true,
    };
  });

  try {
    await bulkPut('device_states', states.map(toPersisted));
  } catch (error) {
    console.warn('Failed to persist device states:', error);
  }

  return states;
}

export const deviceStateMemory = {
  subscribe,
  hydrate,
  ensureDeviceState,
  ensureDeviceStates,
  updateDeviceState,
};

void hydrate();
