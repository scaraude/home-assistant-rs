import { updateDeviceColor, updateDeviceName } from '../api';
import { sensorsMemory } from './sensorsMemory';
import { switchesMemory } from './switchesMemory';

async function setDeviceName(deviceId: string, name: string): Promise<void> {
  await updateDeviceName(deviceId, name);
  sensorsMemory.applySensorName(deviceId, name);
  switchesMemory.applySwitchName(deviceId, name);
}

async function setDeviceColor(deviceId: string, color: string): Promise<void> {
  await updateDeviceColor(deviceId, color);
  sensorsMemory.applySensorColor(deviceId, color);
  switchesMemory.applySwitchColor(deviceId, color);
}

export const devicesMemory = {
  setDeviceName,
  setDeviceColor,
};
