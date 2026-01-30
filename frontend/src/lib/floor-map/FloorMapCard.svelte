<script lang="ts">
  import Badge from "../design-system/Badge.svelte";
  import Icon from "../design-system/Icon.svelte";
  import type { NetworkDevice, DeviceState } from "../types/devices";
  import type { SensorReading, TempHumiditySensorReading, PresenceSensorReading, EnergySensorReading } from "../api";
  import { getLqiBadgeVariant } from "../utils/badge";

  export type FloorMapDeviceType = "temp_humidity" | "presence" | "switch" | "energy_meter" | "coordinator" | "router";

  interface Props {
    device: NetworkDevice;
    deviceState?: DeviceState | null;
    latestReading?: SensorReading | null;
    switchState?: boolean;
    onclick?: () => void;
  }

  let { device, deviceState = null, latestReading = null, switchState = false, onclick }: Props = $props();

  // Determine device type from capabilities
  const deviceType = $derived.by((): FloorMapDeviceType => {
    if (device.capabilities.some((cap) => cap.type === "coordinator")) {
      return "coordinator";
    }
    if (device.is_bridge) {
      return "router";
    }
    const sensorCap = device.capabilities.find((cap) => cap.type === "sensor");
    if (sensorCap?.type === "sensor") {
      if (sensorCap.sensor_type === "temp_humidity") return "temp_humidity";
      if (sensorCap.sensor_type === "presence") return "presence";
      if (sensorCap.sensor_type === "energy_meter") return "energy_meter";
    }
    const commanderCap = device.capabilities.find((cap) => cap.type === "commander");
    if (commanderCap?.type === "commander" && commanderCap.commander_type === "switch") {
      return "switch";
    }
    return "router";
  });

  // Border color based on role
  const borderColor = $derived.by((): string => {
    if (deviceType === "coordinator") return "var(--color-floor-card-border-coordinator)";
    if (deviceType === "router") return "var(--color-floor-card-border-router)";
    return "var(--color-floor-card-border-standard)";
  });

  // Border width based on role
  const borderWidth = $derived(
    deviceType === "coordinator" || deviceType === "router" ? "3px" : "2px"
  );

  // Type guards for readings
  function isTempHumidity(reading: SensorReading): reading is TempHumiditySensorReading {
    return reading.type === "temp_humidity";
  }
  function isPresence(reading: SensorReading): reading is PresenceSensorReading {
    return reading.type === "presence";
  }
  function isEnergy(reading: SensorReading): reading is EnergySensorReading {
    return reading.type === "energy_meter";
  }
</script>

<button
  class="floor-map-card"
  class:coordinator={deviceType === "coordinator"}
  class:router={deviceType === "router"}
  class:switch-on={deviceType === "switch" && switchState}
  style="--border-color: {borderColor}; --border-width: {borderWidth};"
  onclick={onclick}
  type="button"
>
  <!-- LQI Badge -->
  {#if deviceState?.link_quality != null}
    <div class="lqi-badge">
      <Badge variant={getLqiBadgeVariant(deviceState.link_quality)} size="mini">
        {deviceState.link_quality}
      </Badge>
    </div>
  {/if}

  <!-- Icon -->
  <div class="icon-container" class:on={deviceType === "switch" && switchState}>
    {#if deviceType === "temp_humidity"}
      <Icon name="thermometer" size={20} />
    {:else if deviceType === "presence"}
      <div class="presence-indicator" class:occupied={latestReading && isPresence(latestReading) && latestReading.occupied}></div>
    {:else if deviceType === "switch"}
      <Icon name="lightbulb" size={20} />
    {:else if deviceType === "energy_meter"}
      <Icon name="bolt" size={20} />
    {:else if deviceType === "coordinator"}
      <Icon name="coordinator" size={20} />
    {:else}
      <Icon name="signal" size={20} />
    {/if}
  </div>

  <!-- Device Name -->
  <div class="device-name" title={device.name}>
    {device.name}
  </div>

  <!-- Data Display -->
  <div class="data-display">
    {#if deviceType === "temp_humidity" && latestReading && isTempHumidity(latestReading)}
      <span class="primary-value">{latestReading.temperature.toFixed(1)}°C</span>
      <span class="secondary-value">{latestReading.humidity.toFixed(0)}%</span>
    {:else if deviceType === "presence" && latestReading && isPresence(latestReading)}
      <span class="presence-text" class:occupied={latestReading.occupied}>
        {latestReading.occupied ? "Occupied" : "Clear"}
      </span>
    {:else if deviceType === "switch"}
      <span class="switch-state" class:on={switchState}>
        {switchState ? "ON" : "OFF"}
      </span>
    {:else if deviceType === "energy_meter" && latestReading && isEnergy(latestReading)}
      <span class="primary-value">{latestReading.power.toFixed(0)} W</span>
    {:else if deviceType === "coordinator"}
      <span class="role-label">Coordinator</span>
    {:else if deviceType === "router"}
      <span class="role-label">Router</span>
    {/if}
  </div>
</button>

<style>
  .floor-map-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    width: 120px;
    padding: 0.5rem;
    background: var(--color-card-bg);
    border: var(--border-width) solid var(--border-color);
    border-radius: 8px;
    box-shadow: var(--shadow-floor-card);
    cursor: pointer;
    transition: all 0.2s ease;
    font-family: inherit;
  }

  .floor-map-card:hover {
    box-shadow: var(--shadow-floor-card-hover);
    transform: translateY(-2px);
  }

  .floor-map-card:active {
    transform: translateY(0);
  }

  .floor-map-card.coordinator {
    background: linear-gradient(
      135deg,
      var(--color-floor-card-coordinator-start) 0%,
      var(--color-floor-card-coordinator-end) 100%
    );
  }

  .floor-map-card.router {
    background: linear-gradient(
      135deg,
      var(--color-floor-card-router-start) 0%,
      var(--color-floor-card-router-end) 100%
    );
  }

  .floor-map-card.switch-on {
    background: linear-gradient(
      135deg,
      var(--color-floor-card-switch-start) 0%,
      var(--color-floor-card-switch-end) 100%
    );
  }

  /* LQI Badge */
  .lqi-badge {
    position: absolute;
    top: 0.25rem;
    right: 0.25rem;
  }

  /* Icon Container */
  .icon-container {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: var(--color-floor-card-icon-bg);
    border-radius: 8px;
    color: var(--color-floor-card-icon-text);
    transition: all 0.2s ease;
  }

  .icon-container.on {
    background: linear-gradient(
      135deg,
      var(--color-floor-card-icon-on-start) 0%,
      var(--color-floor-card-icon-on-end) 100%
    );
    color: white;
    box-shadow: var(--shadow-floor-card-icon-on);
  }

  /* Presence Indicator */
  .presence-indicator {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--color-card-border-hover);
    transition: all 0.3s ease;
  }

  .presence-indicator.occupied {
    background: linear-gradient(
      135deg,
      var(--color-floor-card-icon-on-start) 0%,
      var(--color-floor-card-icon-on-end) 100%
    );
    box-shadow: var(--shadow-floor-card-presence-on);
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(1.1);
      opacity: 0.8;
    }
  }

  /* Device Name */
  .device-name {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--color-floor-card-text);
    text-align: center;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.2;
  }

  /* Data Display */
  .data-display {
    display: flex;
    align-items: baseline;
    gap: 0.375rem;
    min-height: 1.125rem;
  }

  .primary-value {
    font-size: 0.875rem;
    font-weight: 700;
    color: var(--color-floor-card-text-strong);
  }

  .secondary-value {
    font-size: 0.625rem;
    font-weight: 500;
    color: var(--color-floor-card-text-muted);
  }

  .presence-text {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--color-floor-card-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .presence-text.occupied {
    color: var(--color-floor-card-presence-on);
  }

  .switch-state {
    font-size: 0.6875rem;
    font-weight: 700;
    color: var(--color-floor-card-text-muted);
    text-transform: uppercase;
  }

  .switch-state.on {
    color: var(--color-floor-card-switch-on-text);
  }

  .role-label {
    font-size: 0.5625rem;
    font-weight: 600;
    color: var(--color-floor-card-text-subtle);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
