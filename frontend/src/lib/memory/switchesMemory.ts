import { writable } from 'svelte/store';
import type { SwitchDevice } from '../api/switches';
import { executeCommand, fetchSwitches } from '../api';
import { unixSecondsToDate, dateToUnixSeconds } from '../utils/time';
import { bulkPut, getAll, put } from './indexedDb';
import type { DeviceState } from '../types/devices';

interface SwitchesMemoryState {
  devices: SwitchDevice[];
  byId: Record<string, SwitchDevice>;
  loaded: boolean;
  hydrated: boolean;
}

type PersistedSwitchDevice = Omit<SwitchDevice, 'last_seen'> & { last_seen: number | null };

const initialState: SwitchesMemoryState = {
  devices: [],
  byId: {},
  loaded: false,
  hydrated: false,
};

const { subscribe, update } = writable<SwitchesMemoryState>(initialState);

function toPersistedDevice(device: SwitchDevice): PersistedSwitchDevice {
  return {
    ...device,
    last_seen: device.last_seen ? dateToUnixSeconds(device.last_seen) : null,
  };
}

function fromPersistedDevice(device: PersistedSwitchDevice): SwitchDevice {
  return {
    ...device,
    last_seen: device.last_seen ? unixSecondsToDate(device.last_seen) : null,
  };
}

async function hydrate() {
  try {
    const devices = await getAll<PersistedSwitchDevice>('switch_devices');
    const parsed = devices.map(fromPersistedDevice);
    const byId = parsed.reduce<Record<string, SwitchDevice>>((acc, device) => {
      acc[device.id] = device;
      return acc;
    }, {});
    update((state) => ({
      ...state,
      devices: parsed,
      byId,
      loaded: parsed.length > 0,
      hydrated: true,
    }));
  } catch (error) {
    console.warn('Failed to hydrate switches memory:', error);
    update((state) => ({ ...state, hydrated: true }));
  }
}

async function ensureSwitches(force = false): Promise<SwitchDevice[]> {
  let cached: SwitchDevice[] = [];
  let loaded = false;
  subscribe((state) => {
    cached = state.devices;
    loaded = state.loaded;
  })();

  if (loaded && !force) {
    return cached;
  }

  const devices = await fetchSwitches();
  const byId = devices.reduce<Record<string, SwitchDevice>>((acc, device) => {
    acc[device.id] = device;
    return acc;
  }, {});

  update((state) => ({
    ...state,
    devices,
    byId,
    loaded: true,
  }));

  try {
    await bulkPut('switch_devices', devices.map(toPersistedDevice));
  } catch (error) {
    console.warn('Failed to persist switches:', error);
  }

  return devices;
}

function updateSwitchState(deviceId: string, updates: Partial<SwitchDevice>) {
  update((state) => {
    const existing = state.byId[deviceId];
    if (!existing) return state;

    const updated: SwitchDevice = { ...existing, ...updates };
    const devices = state.devices.map((device) => (device.id === deviceId ? updated : device));

    void put('switch_devices', toPersistedDevice(updated)).catch((error) => {
      console.warn('Failed to persist switch update:', error);
    });

    return {
      ...state,
      devices,
      byId: {
        ...state.byId,
        [deviceId]: updated,
      },
    };
  });
}

function applyDeviceState(deviceId: string, stateUpdate: DeviceState) {
  updateSwitchState(deviceId, {
    battery_level: stateUpdate.battery_level,
    link_quality: stateUpdate.link_quality,
    last_seen: stateUpdate.last_seen,
  });
}

async function sendSwitchCommand(deviceId: string, desiredState: boolean): Promise<void> {
  await executeCommand(deviceId, desiredState);
}

function applySwitchName(deviceId: string, name: string): void {
  updateSwitchState(deviceId, { name });
}

function applySwitchColor(deviceId: string, color: string): void {
  updateSwitchState(deviceId, { color });
}

export const switchesMemory = {
  subscribe,
  hydrate,
  ensureSwitches,
  updateSwitchState,
  applyDeviceState,
  sendSwitchCommand,
  applySwitchName,
  applySwitchColor,
};

void hydrate();
