import type { SensorType } from "./sensors";

export type CommanderType = "switch";
export type PowerSource = "battery" | "plugged";

export type DeviceCapability =
  | { type: "sensor"; sensor_type: SensorType }
  | { type: "commander"; commander_type: CommanderType }
  | { type: "router"; turbo_mode: boolean }
  | { type: "coordinator" };

export interface DeviceInfo {
  device_id: string;
  name: string;
  capabilities: DeviceCapability[];
  available_fields: string[];
  color: string | null;
  /** Zigbee availability reported by Zigbee2MQTT, null when never reported. */
  availability: "online" | "offline" | null;
  /** Unix timestamp (seconds) of the last availability transition. */
  availability_changed_at: number | null;
}

export interface DeviceState {
  device_id: string;
  battery_level: number | null;
  link_quality: number | null;
  last_seen: Date;
  turbo_mode: boolean | null;
}

export interface NetworkDevice {
  id: string;
  mqtt_topic: string;
  ieee_addr: string;
  name: string;
  capabilities: DeviceCapability[];
  available_fields: string[];
  power_source: PowerSource;
  added_at: Date;
  is_bridge: boolean;
  parent_device_id: string | null;
  color: string | null;
}
