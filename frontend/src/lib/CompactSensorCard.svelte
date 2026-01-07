<script lang="ts">
  import ColorPickerModal from "./ColorPickerModal.svelte";
  import { graphConfig } from "./stores/graphConfig";
  import { dataCache } from "./stores/dataCache";
  import { updateDeviceName } from "./api";
  import type { SensorUIConfig } from "./stores/graphConfig";
  import type { SensorReading } from "./api";

  let {
    sensor,
    latestReading,
    editMode = false
  }: {
    sensor: SensorUIConfig;
    latestReading: SensorReading | null;
    editMode?: boolean;
  } = $props();

  let isEditing = $state(false);
  let editedName = $state("");
  let showColorPicker = $state(false);

  // Sync isEditing with editMode prop
  $effect(() => {
    isEditing = editMode;
    if (editMode) {
      editedName = deviceName;
    }
  });

  // Get device name from cache
  let deviceInfo = $derived(
    $dataCache.sensors.devices.find(d => d.device_id === sensor.deviceId)
  );

  let deviceName = $derived(deviceInfo?.name || sensor.deviceId);

  // Get device state from cache (updated via WebSocket)
  let deviceState = $derived($dataCache.deviceStates[sensor.deviceId] || null);

  function handleColorClick() {
    showColorPicker = true;
  }

  function handleColorSelect(color: string) {
    graphConfig.setSensorColor(sensor.deviceId, color);
  }

  function handleCardClick() {
    graphConfig.toggleSensor(sensor.deviceId);
  }

  function handleNameClick(e: MouseEvent) {
    e.stopPropagation();
    // Only allow editing when editMode is active (controlled by parent)
    // Individual clicking to edit is disabled
  }

  async function handleNameSubmit() {
    if (editedName.trim() && editedName !== deviceName) {
      try {
        await updateDeviceName(sensor.deviceId, editedName.trim());
        dataCache.updateDeviceName(sensor.deviceId, editedName.trim());
      } catch (err) {
        console.error("Failed to update device name:", err);
      }
    }
    isEditing = false;
  }

  function handleNameBlur() {
    void handleNameSubmit();
  }

  function handleNameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      void handleNameSubmit();
    } else if (e.key === "Escape") {
      isEditing = false;
    }
  }
</script>

<div
  class="sensor-card"
  class:active={sensor.visible}
  onclick={handleCardClick}
  onkeydown={(e) => e.key === 'Enter' && handleCardClick()}
  role="button"
  tabindex="0"
  aria-pressed={sensor.visible}
>
  {#if deviceState && (deviceState.battery_level !== null || deviceState.link_quality !== null)}
    <div class="corner-badges">
      {#if deviceState.battery_level !== null && deviceState.battery_level !== undefined}
        <span
          class="corner-badge"
          class:battery-low={deviceState.battery_level < 20}
          title="Battery: {deviceState.battery_level}%"
        >
          🔋 {deviceState.battery_level}
        </span>
      {/if}
      {#if deviceState.link_quality !== null && deviceState.link_quality !== undefined}
        <span
          class="corner-badge"
          class:link-weak={deviceState.link_quality < 50}
          title="Link quality: {deviceState.link_quality}%"
        >
          📶 {deviceState.link_quality}
        </span>
      {/if}
    </div>
  {/if}

  <div class="sensor-header">
    <div class="color-name-group">
      <button
        class="color-dot"
        style="background-color: {sensor.color}"
        onclick={(e) => { e.stopPropagation(); handleColorClick(); }}
        aria-label="Change color"
        title="Click to change color"
      ></button>

      {#if isEditing}
        <input
          type="text"
          class="name-input"
          bind:value={editedName}
          onblur={handleNameBlur}
          onkeydown={handleNameKeydown}
          onclick={(e) => e.stopPropagation()}
        />
      {:else}
        <span class="sensor-name">
          {deviceName}
        </span>
      {/if}
    </div>
  </div>

  <div class="sensor-readings">
    {#if latestReading}
      <div class="reading">
        <span class="reading-label">Temp</span>
        <span class="reading-value">{latestReading.temperature.toFixed(1)}°C</span>
      </div>
      <div class="reading">
        <span class="reading-label">Humidity</span>
        <span class="reading-value">{latestReading.humidity.toFixed(0)}%</span>
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
    onClose={() => showColorPicker = false}
  />
{/if}

<style>
  .sensor-card {
    position: relative;
    background: white;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
    padding: 0.875rem;
    cursor: pointer;
    transition: all 0.15s;
    user-select: none;
  }

  .sensor-card:hover {
    border-color: #d1d5db;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  }

  .sensor-card.active {
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

  .corner-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.125rem;
    padding: 0.125rem 0.375rem;
    background: rgba(243, 244, 246, 0.95);
    backdrop-filter: blur(4px);
    border-radius: 4px;
    font-size: 0.625rem;
    color: #4b5563;
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
    line-height: 1;
  }

  .corner-badge.battery-low {
    background: rgba(254, 243, 199, 0.95);
    color: #92400e;
  }

  .corner-badge.link-weak {
    background: rgba(254, 226, 226, 0.95);
    color: #991b1b;
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
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(0, 0, 0, 0.1);
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
    padding: 0;
  }

  .color-dot:hover {
    transform: scale(1.15);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25), 0 0 0 2px rgba(0, 0, 0, 0.15);
  }

  .sensor-name {
    font-size: 0.9375rem;
    font-weight: 600;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .name-input {
    flex: 1;
    padding: 0.25rem 0.5rem;
    border: 1px solid #3b82f6;
    border-radius: 4px;
    font-size: 0.9375rem;
    font-weight: 600;
    color: #111827;
    background: white;
  }

  .name-input:focus {
    outline: none;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .sensor-readings {
    display: flex;
    gap: 0.75rem;
  }

  .reading {
    flex: 1;
    background: #f9fafb;
    padding: 0.5rem 0.625rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .reading-label {
    font-size: 0.6875rem;
    color: #6b7280;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .reading-value {
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
  }

  .no-reading {
    flex: 1;
    text-align: center;
    color: #9ca3af;
    font-size: 0.875rem;
    padding: 0.5rem;
  }
</style>
