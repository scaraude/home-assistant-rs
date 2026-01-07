<script lang="ts">
  import ColorPickerModal from "./ColorPickerModal.svelte";
  import { graphConfig } from "./stores/graphConfig";
  import { dataCache } from "./stores/dataCache";
  import { updateDeviceName } from "./api";
  import type { SensorUIConfig } from "./stores/graphConfig";
  import type { SensorReading } from "./api";

  let {
    sensor,
    latestReading
  }: {
    sensor: SensorUIConfig;
    latestReading: SensorReading | null;
  } = $props();

  let isEditing = $state(false);
  let editedName = $state("");
  let showColorPicker = $state(false);

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
    isEditing = true;
    editedName = deviceName;
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
        <button
          class="sensor-name-btn"
          onclick={handleNameClick}
          title="Click to edit name"
        >
          {deviceName}
        </button>
      {/if}
    </div>

    {#if sensor.visible}
      <span class="active-indicator" aria-label="Visible on graph">✓</span>
    {/if}
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

  {#if deviceState}
    <div class="sensor-badges">
      {#if deviceState.battery_level !== null && deviceState.battery_level !== undefined}
        <span
          class="badge"
          class:battery-low={deviceState.battery_level < 20}
          title="Battery level"
        >
          🔋 {deviceState.battery_level}%
        </span>
      {/if}
      {#if deviceState.link_quality !== null && deviceState.link_quality !== undefined}
        <span
          class="badge"
          class:link-weak={deviceState.link_quality < 50}
          title="Link quality"
        >
          📶 {deviceState.link_quality}%
        </span>
      {/if}
    </div>
  {/if}
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

  .sensor-name-btn {
    margin: 0;
    padding: 0;
    background: none;
    border: none;
    font-size: 0.9375rem;
    font-weight: 600;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
    text-align: left;
    flex: 1;
    min-width: 0;
  }

  .sensor-name-btn:hover {
    color: #3b82f6;
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

  .active-indicator {
    color: #10b981;
    font-size: 1.125rem;
    font-weight: bold;
    line-height: 1;
  }

  .sensor-readings {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 0.625rem;
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

  .sensor-badges {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: #f3f4f6;
    border-radius: 4px;
    font-size: 0.75rem;
    color: #4b5563;
    font-weight: 500;
  }

  .badge.battery-low {
    background: #fef3c7;
    color: #92400e;
  }

  .badge.link-weak {
    background: #fee2e2;
    color: #991b1b;
  }
</style>
