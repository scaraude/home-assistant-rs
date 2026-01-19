export type SensorType = "temp_humidity" | "presence" | "energy_meter";

export type TempHumiditySensorField = "temperature" | "humidity";
export type PresenceSensorField = "presence" | "illumination";
export type EnergyMeterField =
  | "power"
  | "voltage"
  | "current"
  | "energy"
  | "produced_energy"
  | "ac_frequency"
  | "power_factor";

export type SensorField =
  | TempHumiditySensorField
  | PresenceSensorField
  | EnergyMeterField;

export type DeviceStateField = "battery" | "link_quality";

export type AutomationConditionField =
  | TempHumiditySensorField
  | PresenceSensorField
  | DeviceStateField;

export function isTempHumidityField(
  field: string
): field is TempHumiditySensorField {
  return field === "temperature" || field === "humidity";
}

export function isPresenceField(field: string): field is PresenceSensorField {
  return field === "presence" || field === "illumination";
}

export function isDeviceStateField(field: string): field is DeviceStateField {
  return field === "battery" || field === "link_quality";
}
