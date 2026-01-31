<script lang="ts">
  import type { NetworkDevice, DeviceState } from "../types/devices";
  import type {
    SensorReading,
    TempHumiditySensorReading,
    PresenceSensorReading,
    EnergySensorReading,
  } from "../api";
  import StatusBadge from "../shared/StatusBadge.svelte";

  export type FloorMapDeviceType =
    | "temp_humidity"
    | "presence"
    | "switch"
    | "energy_meter"
    | "coordinator"
    | "router";

  interface Props {
    device: NetworkDevice;
    deviceState?: DeviceState | null;
    latestReading?: SensorReading | null;
    switchState?: boolean;
    onclick?: () => void;
    onSwitchToggle?: (deviceId: string, newState: boolean) => void;
    onTurboToggle?: (deviceId: string, newState: boolean) => void;
  }

  let {
    device,
    deviceState = null,
    latestReading = null,
    switchState = false,
    onclick,
    onSwitchToggle,
    onTurboToggle,
  }: Props = $props();

  function handleSwitchClick(event: MouseEvent) {
    event.stopPropagation();
    onSwitchToggle?.(device.id, !switchState);
  }

  function handleTurboClick(event: MouseEvent) {
    event.stopPropagation();
    onTurboToggle?.(device.id, !deviceState?.turbo_mode);
  }

  // Determine device type from capabilities
  const deviceType = $derived.by((): FloorMapDeviceType => {
    if (device.capabilities.some((cap) => cap.type === "coordinator")) {
      return "coordinator";
    }
    const commanderCap = device.capabilities.find(
      (cap) => cap.type === "commander",
    );
    if (
      commanderCap?.type === "commander" &&
      commanderCap.commander_type === "switch"
    ) {
      return "switch";
    }
    const sensorCap = device.capabilities.find((cap) => cap.type === "sensor");
    if (sensorCap?.type === "sensor") {
      if (sensorCap.sensor_type === "temp_humidity") return "temp_humidity";
      if (sensorCap.sensor_type === "presence") return "presence";
      if (sensorCap.sensor_type === "energy_meter") return "energy_meter";
    }
    if (device.is_bridge) {
      return "router";
    }
    return "router";
  });

  const hasRouterCapability = $derived.by((): boolean => {
    return (
      device.is_bridge ||
      device.capabilities.some((cap) => cap.type === "router")
    );
  });

  const isTempHumidityCard = $derived(deviceType === "temp_humidity");
  const isEnergyCard = $derived(deviceType === "energy_meter");
  const isPresenceCard = $derived(deviceType === "presence");
  const isSwitchCard = $derived(deviceType === "switch");
  const isCoordinatorCard = $derived(deviceType === "coordinator");
  const isRouterCard = $derived(deviceType === "router");
  const showTurbo = $derived(isSwitchCard && hasRouterCapability);

  // Type guards for readings
  function isTempHumidity(
    reading: SensorReading,
  ): reading is TempHumiditySensorReading {
    return reading.type === "temp_humidity";
  }
  function isPresence(
    reading: SensorReading,
  ): reading is PresenceSensorReading {
    return reading.type === "presence";
  }
  function isEnergy(reading: SensorReading): reading is EnergySensorReading {
    return reading.type === "energy_meter";
  }

  const tempValue = $derived.by(() => {
    if (latestReading && isTempHumidity(latestReading)) {
      return latestReading.temperature.toFixed(1);
    }
    return "--";
  });

  const humidityValue = $derived.by(() => {
    if (latestReading && isTempHumidity(latestReading)) {
      return latestReading.humidity.toFixed(0);
    }
    return "--";
  });

  const energyValue = $derived.by(() => {
    if (latestReading && isEnergy(latestReading)) {
      return latestReading.power.toFixed(0);
    }
    return "--";
  });

  const presenceActive = $derived.by(() => {
    return Boolean(
      latestReading && isPresence(latestReading) && latestReading.occupied,
    );
  });

  const switchActive = $derived(isSwitchCard && switchState);
  const turboActive = $derived(Boolean(deviceState?.turbo_mode));
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="floor-map-card"
  class:pill={isTempHumidityCard ||
    isEnergyCard ||
    isSwitchCard ||
    isRouterCard}
  class:square={isPresenceCard || isCoordinatorCard}
  class:coordinator={isCoordinatorCard}
  class:router-outline={hasRouterCapability}
  class:turbo-on={turboActive}
  class:is-switch={isSwitchCard}
  {onclick}
  onkeydown={isSwitchCard ? undefined : (e) => e.key === "Enter" && onclick?.()}
  role={isSwitchCard ? undefined : "button"}
  tabindex={isSwitchCard ? undefined : 0}
>
  {#if deviceState?.link_quality != null}
    <div class="container-badge">
      <StatusBadge type="signal" value={deviceState.link_quality} mini />
    </div>
  {/if}

  {#if isTempHumidityCard}
    <div class="pill-content">
      <div class="icon-chip" aria-hidden="true">🌡️</div>
      <div class="pill-values temp-values">
        <span class="value-strong">{tempValue}°C</span>
        <span class="divider">|</span>
        <span class="value-muted">{humidityValue}%</span>
      </div>
    </div>
  {:else if isEnergyCard}
    <div class="pill-content">
      <div class="icon-chip energy" aria-hidden="true">⚡️</div>
      <div class="pill-values">
        <span class="value-strong">{energyValue} W</span>
      </div>
    </div>
  {:else if isPresenceCard}
    <div class="square-content">
      <div class="presence-emoji" aria-hidden="true">
        {presenceActive ? "🟡" : "⚪️"}
      </div>
      <div class="square-label">{device.name}</div>
    </div>
  {:else if isCoordinatorCard}
    <div class="square-content">
      <div class="icon-chip coordinator" aria-hidden="true">📡</div>
      <div class="square-label">{device.name}</div>
    </div>
  {:else}
    <div class="pill-content switch-layout" class:has-turbo={showTurbo}>
      <button
        class="icon-square"
        class:on={switchActive}
        class:router={hasRouterCapability}
        onclick={handleSwitchClick}
        type="button"
        aria-label={switchActive ? "Turn off" : "Turn on"}
      >
        ⚙️
      </button>
      <div class="switch-name" title={device.name}>
        {device.name}
      </div>
      {#if showTurbo}
        <button
          class="turbo"
          onclick={handleTurboClick}
          type="button"
          aria-label={turboActive ? "Disable turbo" : "Enable turbo"}
        >
          <span>Turbo</span>
          <div class="toggle" class:on={turboActive}>
            <span class="toggle-knob"></span>
          </div>
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .floor-map-card {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem 0.75rem;
    background: #d7d7d7;
    border: 2px solid #141414;
    border-radius: 14px;
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.2);
    cursor: pointer;
    transition:
      transform var(--transition-fast),
      box-shadow var(--transition-fast),
      border-color var(--transition-fast);
    font-family: inherit;
  }

  .floor-map-card.is-switch {
    cursor: default;
  }

  .floor-map-card:hover {
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
    transform: translateY(-2px);
  }

  .floor-map-card:active {
    transform: translateY(0);
  }

  .floor-map-card.pill {
    width: 240px;
    height: 60px;
  }

  .floor-map-card.square {
    width: 90px;
    height: 90px;
    padding: 0.5rem;
  }

  .floor-map-card.turbo-on .toggle {
    background: #6bcf5b;
  }

  .floor-map-card.coordinator {
    border-color: #c26a00;
  }

  .floor-map-card.router-outline {
    border-color: #1d4ed8;
    box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.5);
  }

  .container-badge {
    position: absolute;
    top: -4px;
    right: 4px;
  }

  .pill-content {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }

  .pill-values {
    display: flex;
    align-items: baseline;
    gap: 10px;
    font-size: 28px;
    font-weight: 600;
    color: #111111;
    letter-spacing: -0.02em;
  }

  .pill-values.temp-values {
    gap: 8px;
  }

  .value-strong {
    font-size: 30px;
    font-weight: 700;
  }

  .value-muted {
    font-size: 30px;
    font-weight: 500;
  }

  .divider {
    font-size: 26px;
    font-weight: 500;
    color: #1f1f1f;
    opacity: 0.8;
  }

  .icon-chip {
    width: 60px;
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #111111;
    font-size: 28px;
  }

  .icon-chip.energy {
    color: #fbbf24;
  }

  .icon-chip.coordinator {
    color: #4b4b4b;
    border-radius: 10px;
  }

  .square-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .square-label {
    font-size: 12px;
    font-weight: 600;
    color: #2b2b2b;
    text-align: center;
    line-height: 1.1;
  }

  .presence-emoji {
    width: 26px;
    height: 26px;
    border-radius: 999px;
    border: 2px solid #2b2b2b;
    box-shadow: 1px 1px 0 rgba(0, 0, 0, 0.1);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 26px;
    line-height: 1;
  }

  .switch-layout {
    justify-content: space-between;
  }

  .icon-square {
    width: 38px;
    height: 38px;
    border-radius: 8px;
    background: #4b4b4b;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #d1d5db;
    box-shadow: inset 0 0 0 1px #2b2b2b;
    font-size: 28px;
    cursor: pointer;
    transition:
      transform var(--transition-fast),
      box-shadow var(--transition-fast);
  }

  .icon-square:hover {
    transform: scale(1.1);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .icon-square:active {
    transform: scale(0.95);
  }

  .icon-square.on {
    background: #fbec5d;
    color: #2b2b2b;
  }

  .switch-name {
    flex: 1;
    font-size: 18px;
    font-weight: 600;
    color: #111111;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .turbo {
    position: absolute;
    right: 10px;
    bottom: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #2b2b2b;
    background: none;
    border: none;
    padding: 4px;
    border-radius: 6px;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .turbo:hover {
    background: rgba(0, 0, 0, 0.08);
  }

  .turbo:active {
    background: rgba(0, 0, 0, 0.12);
  }

  .switch-layout.has-turbo {
    padding-right: 68px;
  }

  .toggle {
    width: 46px;
    height: 22px;
    border-radius: 999px;
    background: #d1d5db;
    position: relative;
    transition: background var(--transition-fast);
  }

  .toggle.on {
    background: #6bcf5b;
  }

  .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #ffffff;
    transition: transform var(--transition-fast);
  }

  .toggle.on .toggle-knob {
    transform: translateX(24px);
  }

  @media (max-width: 720px) {
    .floor-map-card.pill {
      width: 210px;
      height: 56px;
    }

    .pill-values,
    .value-strong,
    .value-muted {
      font-size: 24px;
    }

    .switch-name {
      font-size: 22px;
    }
  }
</style>
