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

  // Temperature color gradients based on value
  const tempGradient = $derived.by(() => {
    const temp = latestReading && isTempHumidity(latestReading)
      ? latestReading.temperature
      : 20;
    if (temp < 16) return 'var(--card-gradient-cold)';
    if (temp < 20) return 'var(--card-gradient-cool)';
    if (temp < 24) return 'var(--card-gradient-warm)';
    return 'var(--card-gradient-hot)';
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="floor-card"
  class:pill={isTempHumidityCard || isEnergyCard || isSwitchCard || isRouterCard}
  class:square={isPresenceCard || isCoordinatorCard}
  class:coordinator={isCoordinatorCard}
  class:router={hasRouterCapability}
  class:turbo-active={turboActive}
  class:is-switch={isSwitchCard}
  class:switch-on={switchActive}
  class:presence-active={presenceActive}
  {onclick}
  onkeydown={isSwitchCard ? undefined : (e) => e.key === "Enter" && onclick?.()}
  role={isSwitchCard ? undefined : "button"}
  tabindex={isSwitchCard ? undefined : 0}
>
  <!-- Decorative glow layer -->
  <div class="card-glow"></div>

  <!-- Glass overlay -->
  <div class="card-glass"></div>

  <!-- Signal badge -->
  {#if deviceState?.link_quality != null}
    <div class="signal-badge">
      <StatusBadge type="signal" value={deviceState.link_quality} mini />
    </div>
  {/if}

  <!-- Content layer -->
  <div class="card-content">
    {#if isTempHumidityCard}
      <div class="sensor-layout">
        <div class="sensor-icon temp-icon" style="background: {tempGradient}">
          <svg viewBox="0 0 24 24" fill="currentColor" class="icon-svg">
            <path d="M15 13V5c0-1.66-1.34-3-3-3S9 3.34 9 5v8c-1.21.91-2 2.37-2 4 0 2.76 2.24 5 5 5s5-2.24 5-5c0-1.63-.79-3.09-2-4zm-4-8c0-.55.45-1 1-1s1 .45 1 1h-1v1h1v2h-1v1h1v2h-2V5z"/>
          </svg>
        </div>
        <div class="sensor-values">
          <div class="value-row">
            <span class="value-primary">{tempValue}</span>
            <span class="value-unit">°C</span>
          </div>
          <div class="value-secondary">
            <svg viewBox="0 0 24 24" fill="currentColor" class="humidity-icon">
              <path d="M12 2c-5.33 4.55-8 8.48-8 11.8 0 4.98 3.8 8.2 8 8.2s8-3.22 8-8.2c0-3.32-2.67-7.25-8-11.8zm0 18c-3.35 0-6-2.57-6-6.2 0-2.34 1.95-5.44 6-9.14 4.05 3.7 6 6.79 6 9.14 0 3.63-2.65 6.2-6 6.2z"/>
            </svg>
            <span>{humidityValue}%</span>
          </div>
        </div>
      </div>
    {:else if isEnergyCard}
      <div class="sensor-layout">
        <div class="sensor-icon energy-icon">
          <svg viewBox="0 0 24 24" fill="currentColor" class="icon-svg pulse">
            <path d="M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.07-.12C8.48 10.94 10.42 7.54 13 3h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z"/>
          </svg>
        </div>
        <div class="sensor-values">
          <div class="value-row">
            <span class="value-primary">{energyValue}</span>
            <span class="value-unit">W</span>
          </div>
          <div class="value-label">Power</div>
        </div>
      </div>
    {:else if isPresenceCard}
      <div class="presence-layout">
        <div class="presence-orb" class:active={presenceActive}>
          <div class="orb-ring"></div>
          <div class="orb-ring delay-1"></div>
          <div class="orb-ring delay-2"></div>
          <div class="orb-core"></div>
        </div>
        <div class="presence-label">{device.name}</div>
      </div>
    {:else if isCoordinatorCard}
      <div class="coordinator-layout">
        <div class="coordinator-icon">
          <div class="signal-wave"></div>
          <div class="signal-wave delay-1"></div>
          <div class="signal-wave delay-2"></div>
          <svg viewBox="0 0 24 24" fill="currentColor" class="icon-svg">
            <path d="M12 5c-3.87 0-7 3.13-7 7h2c0-2.76 2.24-5 5-5s5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4C5.93 1 1 5.93 1 12h2c0-4.97 4.03-9 9-9s9 4.03 9 9h2c0-6.07-4.93-11-11-11zm0 8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z"/>
          </svg>
        </div>
        <div class="coordinator-label">{device.name}</div>
      </div>
    {:else}
      <!-- Switch / Router card -->
      <div class="switch-layout" class:has-turbo={showTurbo}>
        <button
          class="switch-button"
          class:on={switchActive}
          onclick={handleSwitchClick}
          type="button"
          aria-label={switchActive ? "Turn off" : "Turn on"}
        >
          <div class="switch-glow"></div>
          <svg viewBox="0 0 24 24" fill="currentColor" class="switch-icon">
            <path d="M13 3h-2v10h2V3zm4.83 2.17l-1.42 1.42C17.99 7.86 19 9.81 19 12c0 3.87-3.13 7-7 7s-7-3.13-7-7c0-2.19 1.01-4.14 2.58-5.42L6.17 5.17C4.23 6.82 3 9.26 3 12c0 4.97 4.03 9 9 9s9-4.03 9-9c0-2.74-1.23-5.18-3.17-6.83z"/>
          </svg>
        </button>
        <div class="switch-info">
          <div class="switch-name" title={device.name}>{device.name}</div>
          <div class="switch-status" class:on={switchActive}>
            {switchActive ? 'On' : 'Off'}
          </div>
        </div>
        {#if showTurbo}
          <button
            class="turbo-toggle"
            class:active={turboActive}
            onclick={handleTurboClick}
            type="button"
            aria-label={turboActive ? "Disable turbo" : "Enable turbo"}
          >
            <span class="turbo-label">Turbo</span>
            <div class="turbo-track">
              <div class="turbo-knob"></div>
            </div>
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  /* ===== Design Tokens ===== */
  .floor-card {
    --card-bg: linear-gradient(145deg, #fefefe 0%, #f5f5f7 100%);
    --card-border: rgba(0, 0, 0, 0.06);
    --card-shadow:
      0 2px 8px rgba(0, 0, 0, 0.04),
      0 8px 24px rgba(0, 0, 0, 0.06);
    --card-shadow-hover:
      0 4px 12px rgba(0, 0, 0, 0.06),
      0 16px 40px rgba(0, 0, 0, 0.1);

    /* Temperature gradients */
    --card-gradient-cold: linear-gradient(135deg, #a5d8ff 0%, #74c0fc 100%);
    --card-gradient-cool: linear-gradient(135deg, #99e9f2 0%, #66d9e8 100%);
    --card-gradient-warm: linear-gradient(135deg, #ffd43b 0%, #fab005 100%);
    --card-gradient-hot: linear-gradient(135deg, #ff8787 0%, #fa5252 100%);

    /* Accent colors */
    --accent-energy: linear-gradient(135deg, #ffd43b 0%, #f59f00 100%);
    --accent-coordinator: linear-gradient(135deg, #f59f00 0%, #e67700 100%);
    --accent-router: linear-gradient(135deg, #4dabf7 0%, #228be6 100%);
    --accent-switch-on: linear-gradient(135deg, #8ce99a 0%, #51cf66 100%);
    --accent-switch-off: linear-gradient(135deg, #dee2e6 0%, #ced4da 100%);
    --accent-presence: linear-gradient(135deg, #ffd43b 0%, #f59f00 100%);

    /* Typography */
    --font-display: 'SF Pro Display', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    --font-value: 'SF Pro Rounded', 'SF Pro Display', -apple-system, sans-serif;
  }

  /* ===== Base Card ===== */
  .floor-card {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 20px;
    box-shadow: var(--card-shadow);
    cursor: pointer;
    transition:
      transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 0.3s ease;
    font-family: var(--font-display);
    overflow: hidden;
    isolation: isolate;
  }

  .floor-card.is-switch {
    cursor: default;
  }

  .floor-card:hover {
    transform: translateY(-4px) scale(1.02);
    box-shadow: var(--card-shadow-hover);
  }

  .floor-card:active {
    transform: translateY(-2px) scale(1.01);
  }

  /* Card shapes */
  .floor-card.pill {
    width: 240px;
    height: 72px;
    padding: 12px 16px;
  }

  .floor-card.square {
    width: 100px;
    height: 100px;
    padding: 12px;
    border-radius: 24px;
  }

  /* ===== Decorative Layers ===== */
  .card-glow {
    position: absolute;
    inset: -50%;
    background: radial-gradient(
      circle at 30% 30%,
      rgba(255, 255, 255, 0.8) 0%,
      transparent 50%
    );
    pointer-events: none;
    opacity: 0.6;
    z-index: 0;
  }

  .card-glass {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      135deg,
      rgba(255, 255, 255, 0.4) 0%,
      rgba(255, 255, 255, 0.1) 50%,
      transparent 100%
    );
    border-radius: inherit;
    pointer-events: none;
    z-index: 1;
  }

  .card-content {
    position: relative;
    z-index: 2;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
  }

  /* ===== Special Card States ===== */
  .floor-card.coordinator {
    border-color: rgba(245, 159, 0, 0.3);
    background: linear-gradient(145deg, #fffbeb 0%, #fef3c7 100%);
  }

  .floor-card.coordinator .card-glow {
    background: radial-gradient(
      circle at 50% 50%,
      rgba(245, 159, 0, 0.2) 0%,
      transparent 60%
    );
  }

  .floor-card.router {
    border-color: rgba(59, 130, 246, 0.25);
  }

  .floor-card.router::before {
    content: '';
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.3), rgba(99, 102, 241, 0.2));
    border-radius: 22px;
    z-index: -1;
    opacity: 0.6;
  }

  .floor-card.switch-on {
    background: linear-gradient(145deg, #f0fdf4 0%, #dcfce7 100%);
    border-color: rgba(34, 197, 94, 0.25);
  }

  .floor-card.turbo-active::after {
    content: '';
    position: absolute;
    inset: -4px;
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.4), rgba(168, 85, 247, 0.3));
    border-radius: 24px;
    z-index: -1;
    animation: turbo-pulse 2s ease-in-out infinite;
  }

  .floor-card.presence-active {
    background: linear-gradient(145deg, #fffbeb 0%, #fef3c7 100%);
    border-color: rgba(245, 159, 0, 0.3);
  }

  /* ===== Signal Badge ===== */
  .signal-badge {
    position: absolute;
    top: 6px;
    right: 8px;
    z-index: 10;
  }

  /* ===== Temperature/Humidity Layout ===== */
  .sensor-layout {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
  }

  .sensor-icon {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow:
      0 2px 8px rgba(0, 0, 0, 0.1),
      inset 0 1px 0 rgba(255, 255, 255, 0.3);
  }

  .sensor-icon .icon-svg {
    width: 26px;
    height: 26px;
    color: white;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.2));
  }

  .temp-icon {
    background: var(--card-gradient-warm);
  }

  .energy-icon {
    background: var(--accent-energy);
  }

  .energy-icon .pulse {
    animation: icon-pulse 2s ease-in-out infinite;
  }

  .sensor-values {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .value-row {
    display: flex;
    align-items: baseline;
    gap: 2px;
  }

  .value-primary {
    font-family: var(--font-value);
    font-size: 32px;
    font-weight: 600;
    color: #1a1a2e;
    letter-spacing: -0.02em;
    line-height: 1;
  }

  .value-unit {
    font-size: 18px;
    font-weight: 500;
    color: #64748b;
  }

  .value-secondary {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #64748b;
    font-size: 15px;
    font-weight: 500;
  }

  .humidity-icon {
    width: 14px;
    height: 14px;
    opacity: 0.7;
  }

  .value-label {
    font-size: 13px;
    color: #94a3b8;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* ===== Presence Layout ===== */
  .presence-layout {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: 100%;
  }

  .presence-orb {
    position: relative;
    width: 42px;
    height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .orb-core {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, #e2e8f0 0%, #cbd5e1 100%);
    box-shadow:
      inset 0 2px 4px rgba(255, 255, 255, 0.5),
      0 2px 8px rgba(0, 0, 0, 0.1);
    transition: all 0.4s ease;
  }

  .presence-orb.active .orb-core {
    background: var(--accent-presence);
    box-shadow:
      inset 0 2px 4px rgba(255, 255, 255, 0.4),
      0 0 20px rgba(245, 159, 0, 0.5);
  }

  .orb-ring {
    position: absolute;
    inset: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  .presence-orb.active .orb-ring {
    border-color: rgba(245, 159, 0, 0.4);
    opacity: 1;
    animation: orb-ripple 2s ease-out infinite;
  }

  .presence-orb.active .orb-ring.delay-1 {
    animation-delay: 0.5s;
  }

  .presence-orb.active .orb-ring.delay-2 {
    animation-delay: 1s;
  }

  .presence-label {
    font-size: 12px;
    font-weight: 600;
    color: #475569;
    text-align: center;
    line-height: 1.2;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ===== Coordinator Layout ===== */
  .coordinator-layout {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .coordinator-icon {
    position: relative;
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .coordinator-icon .icon-svg {
    width: 28px;
    height: 28px;
    color: #d97706;
    position: relative;
    z-index: 1;
  }

  .signal-wave {
    position: absolute;
    inset: 0;
    border: 2px solid rgba(217, 119, 6, 0.3);
    border-radius: 50%;
    animation: signal-broadcast 2.5s ease-out infinite;
  }

  .signal-wave.delay-1 {
    animation-delay: 0.8s;
  }

  .signal-wave.delay-2 {
    animation-delay: 1.6s;
  }

  .coordinator-label {
    font-size: 11px;
    font-weight: 600;
    color: #92400e;
    text-align: center;
    line-height: 1.2;
  }

  /* ===== Switch Layout ===== */
  .switch-layout {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }

  .switch-layout.has-turbo {
    padding-right: 4px;
  }

  .switch-button {
    position: relative;
    width: 46px;
    height: 46px;
    border-radius: 14px;
    border: none;
    background: var(--accent-switch-off);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
    overflow: hidden;
    flex-shrink: 0;
  }

  .switch-button:hover {
    transform: scale(1.08);
  }

  .switch-button:active {
    transform: scale(0.95);
  }

  .switch-button.on {
    background: var(--accent-switch-on);
  }

  .switch-glow {
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at 30% 30%,
      rgba(255, 255, 255, 0.5) 0%,
      transparent 60%
    );
    opacity: 0.8;
  }

  .switch-button.on .switch-glow {
    animation: glow-pulse 2s ease-in-out infinite;
  }

  .switch-icon {
    width: 24px;
    height: 24px;
    color: #64748b;
    position: relative;
    z-index: 1;
    transition: color 0.3s ease;
  }

  .switch-button.on .switch-icon {
    color: #166534;
  }

  .switch-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .switch-name {
    font-size: 15px;
    font-weight: 600;
    color: #1e293b;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .switch-status {
    font-size: 12px;
    font-weight: 500;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .switch-status.on {
    color: #16a34a;
  }

  /* ===== Turbo Toggle ===== */
  .turbo-toggle {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: 10px;
    transition: background 0.2s ease;
  }

  .turbo-toggle:hover {
    background: rgba(0, 0, 0, 0.04);
  }

  .turbo-label {
    font-size: 10px;
    font-weight: 600;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .turbo-track {
    width: 36px;
    height: 20px;
    border-radius: 10px;
    background: linear-gradient(135deg, #e2e8f0 0%, #cbd5e1 100%);
    position: relative;
    transition: background 0.3s ease;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .turbo-toggle.active .turbo-track {
    background: linear-gradient(135deg, #a78bfa 0%, #8b5cf6 100%);
  }

  .turbo-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .turbo-toggle.active .turbo-knob {
    transform: translateX(16px);
  }

  /* ===== Animations ===== */
  @keyframes orb-ripple {
    0% {
      transform: scale(0.5);
      opacity: 1;
    }
    100% {
      transform: scale(2);
      opacity: 0;
    }
  }

  @keyframes signal-broadcast {
    0% {
      transform: scale(0.6);
      opacity: 0.8;
    }
    100% {
      transform: scale(1.8);
      opacity: 0;
    }
  }

  @keyframes icon-pulse {
    0%, 100% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.1);
    }
  }

  @keyframes glow-pulse {
    0%, 100% {
      opacity: 0.8;
    }
    50% {
      opacity: 0.4;
    }
  }

  @keyframes turbo-pulse {
    0%, 100% {
      opacity: 0.6;
      transform: scale(1);
    }
    50% {
      opacity: 0.8;
      transform: scale(1.02);
    }
  }

  /* ===== Responsive ===== */
  @media (max-width: 720px) {
    .floor-card.pill {
      width: 210px;
      height: 66px;
      padding: 10px 14px;
    }

    .sensor-icon {
      width: 42px;
      height: 42px;
      border-radius: 12px;
    }

    .sensor-icon .icon-svg {
      width: 22px;
      height: 22px;
    }

    .value-primary {
      font-size: 26px;
    }

    .value-unit {
      font-size: 15px;
    }

    .switch-button {
      width: 40px;
      height: 40px;
    }

    .switch-name {
      font-size: 14px;
    }
  }
</style>
