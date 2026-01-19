<script lang="ts">
  import ColorPickerModal from "../shared/ColorPickerModal.svelte";
  import EditableDeviceName from "./EditableDeviceName.svelte";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import { graphConfig } from "../stores/graphConfig";
  import { dataCache } from "../stores/dataCache";
  import { formatDistanceToNow } from "date-fns";
  import type { SensorUIConfig } from "../stores/graphConfig";
  import type { PresenceSensorReading } from "../api";

  let {
    sensor,
    latestReading,
    editMode = false,
  }: {
    sensor: SensorUIConfig;
    latestReading: PresenceSensorReading | null;
    editMode?: boolean;
  } = $props();

  let showColorPicker = $state(false);
  let isEditing = $state(false);

  $effect(() => {
    isEditing = editMode;
  });

  let deviceInfo = $derived(
    $dataCache.sensors.devices.find((d) => d.device_id === sensor.deviceId)
  );

  let deviceName = $derived(deviceInfo?.name || sensor.deviceId);
  let deviceState = $derived($dataCache.deviceStates[sensor.deviceId] || null);

  let timeAgo = $derived(
    latestReading
      ? formatDistanceToNow(new Date(latestReading.timestamp * 1000), {
          addSuffix: true,
        })
      : ""
  );

  // Replace "less than a minute ago" with "now"
  let displayTimeAgo = $derived(
    timeAgo === "less than a minute ago" ? "now" : timeAgo
  );

  function handleColorClick() {
    showColorPicker = true;
  }

  function handleColorSelect(color: string) {
    graphConfig.setSensorColor(sensor.deviceId, color);
  }

  function handleCardClick() {
    graphConfig.toggleSensor(sensor.deviceId);
  }
</script>

<div
  class="presence-card"
  class:active={sensor.visible}
  onclick={handleCardClick}
  onkeydown={(e) => e.key === "Enter" && handleCardClick()}
  role="button"
  tabindex="0"
  aria-pressed={sensor.visible}
>
  {#if deviceState && (deviceState.battery_level != null || deviceState.link_quality != null)}
    <div class="corner-badges">
      {#if deviceState.battery_level != null}
        <StatusBadge type="battery" value={deviceState.battery_level} mini />
      {/if}
      {#if deviceState.link_quality != null}
        <StatusBadge type="signal" value={deviceState.link_quality} mini />
      {/if}
    </div>
  {/if}

  <div class="sensor-header">
    <div class="color-name-group">
      <button
        class="color-dot"
        style="background-color: {sensor.color}"
        onclick={(e) => {
          e.stopPropagation();
          handleColorClick();
        }}
        aria-label="Change color"
        title="Click to change color"
      ></button>

      {#if isEditing}
        <EditableDeviceName
          deviceId={sensor.deviceId}
          name={deviceName}
          onSaved={() => isEditing = false}
        />
      {:else}
        <div class="name-time">
          <span class="sensor-name">{deviceName}</span>
          {#if displayTimeAgo}
            <span class="time-ago">{displayTimeAgo}</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="presence-indicator">
    {#if latestReading}
      <div class="light-bubble" class:occupied={latestReading.occupied}>
        <div class="light-glow"></div>
        <div class="light-core"></div>
      </div>
      <div class="presence-info">
        <div class="presence-label">
          {latestReading.occupied ? "Occupied" : "Clear"}
        </div>
        {#if latestReading.illumination}
          <div class="illumination-badge" class:bright={latestReading.illumination === 'bright'}>
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="5"></circle>
              <line x1="12" y1="1" x2="12" y2="3"></line>
              <line x1="12" y1="21" x2="12" y2="23"></line>
              <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
              <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
              <line x1="1" y1="12" x2="3" y2="12"></line>
              <line x1="21" y1="12" x2="23" y2="12"></line>
              <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
              <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
            </svg>
            {latestReading.illumination === 'bright' ? 'Bright' : 'Dim'}
          </div>
        {/if}
      </div>
    {:else}
      <div class="no-reading">No data</div>
    {/if}
  </div>
</div>

{#if showColorPicker}
  <ColorPickerModal
    currentColor={sensor.color}
    onSelect={handleColorSelect}
    onClose={() => (showColorPicker = false)}
  />
{/if}

<style>
  .presence-card {
    position: relative;
    background: white;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
    padding: 0.875rem;
    cursor: pointer;
    transition: all 0.15s;
    user-select: none;
  }

  .presence-card:hover {
    border-color: #d1d5db;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  }

  .presence-card.active {
    border-color: #3b82f6;
    background: #eff6ff;
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.15);
  }

  .corner-badges {
    position: absolute;
    top: 0.375rem;
    right: 0.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    align-items: flex-end;
    z-index: 1;
  }

  .sensor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .color-name-group {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    flex: 1;
    min-width: 0;
  }

  .color-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid white;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.2),
      0 0 0 1px rgba(0, 0, 0, 0.1);
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
    padding: 0;
  }

  .color-dot:hover {
    transform: scale(1.15);
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.25),
      0 0 0 2px rgba(0, 0, 0, 0.15);
  }

  .name-time {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .sensor-name {
    font-size: 0.9375rem;
    font-weight: 600;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time-ago {
    font-size: 0.625rem;
    color: #9ca3af;
  }

  .presence-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 0;
  }

  .light-bubble {
    position: relative;
    width: 60px;
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .light-glow {
    position: absolute;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(156, 163, 175, 0.4) 0%, rgba(156, 163, 175, 0) 70%);
    transition: all 0.3s ease;
  }

  .light-bubble.occupied .light-glow {
    background: radial-gradient(circle, rgba(251, 191, 36, 0.6) 0%, rgba(251, 191, 36, 0.2) 50%, rgba(251, 191, 36, 0) 70%);
    animation: pulse 2s ease-in-out infinite;
  }

  .light-core {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: #d1d5db;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
    transition: all 0.3s ease;
  }

  .light-bubble.occupied .light-core {
    background: linear-gradient(135deg, #fcd34d 0%, #f59e0b 100%);
    box-shadow:
      0 0 20px rgba(251, 191, 36, 0.6),
      inset 0 2px 4px rgba(255, 255, 255, 0.5);
  }

  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(1.1);
      opacity: 0.8;
    }
  }

  .presence-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.375rem;
  }

  .presence-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .light-bubble.occupied ~ .presence-info .presence-label {
    color: #f59e0b;
  }

  .illumination-badge {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: #f3f4f6;
    border-radius: 4px;
    font-size: 0.6875rem;
    font-weight: 500;
    color: #6b7280;
    text-transform: capitalize;
  }

  .illumination-badge.bright {
    background: #fef3c7;
    color: #f59e0b;
  }

  .illumination-badge svg {
    flex-shrink: 0;
  }

  .no-reading {
    text-align: center;
    color: #9ca3af;
    font-size: 0.875rem;
    padding: 0.5rem;
  }
</style>
