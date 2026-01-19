<script lang="ts">
  import ColorPickerModal from "../shared/ColorPickerModal.svelte";
  import Card from "../design-system/Card.svelte";
  import EditableDeviceName from "./EditableDeviceName.svelte";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import { graphConfig } from "../stores/graphConfig";
  import { dataCache } from "../stores/dataCache";
  import { formatDistanceToNow } from "date-fns";
  import type { SensorUIConfig } from "../stores/graphConfig";
  import type { EnergySensorReading } from "../api";

  let {
    sensor,
    latestReading,
    editMode = false,
  }: {
    sensor: SensorUIConfig;
    latestReading: EnergySensorReading | null;
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
      ? formatDistanceToNow(latestReading.timestamp, {
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

<Card
  active={sensor.visible}
  interactive
  onclick={handleCardClick}
  onkeydown={(e: KeyboardEvent) => e.key === "Enter" && handleCardClick()}
  role="button"
  tabindex="0"
  aria-pressed={sensor.visible}
>
  {#if deviceState && deviceState.link_quality != null}
    <div class="corner-badges">
      <StatusBadge type="signal" value={deviceState.link_quality} mini />
    </div>
  {/if}

  <div class="energy-header">
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
          onSaved={() => (isEditing = false)}
        />
      {:else}
        <div class="name-time">
          <span class="energy-name">{deviceName}</span>
          {#if displayTimeAgo}
            <span class="time-ago">{displayTimeAgo}</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="energy-readings">
    {#if latestReading}
      <!-- Primary Metric: Power -->
      <div class="reading primary">
        <span class="reading-label">Power</span>
        <span class="reading-value">{latestReading.power.toFixed(0)} W</span>
      </div>

      <!-- Secondary Metrics: Energy (conditionally shown) -->
      {#if latestReading.energy > 0}
        <div class="reading secondary">
          <span class="reading-label">Energy</span>
          <span class="reading-value">{latestReading.energy.toFixed(2)} kWh</span>
        </div>
      {/if}

      {#if latestReading.produced_energy > 0}
        <div class="reading secondary">
          <span class="reading-label">Produced</span>
          <span class="reading-value">{latestReading.produced_energy.toFixed(2)} kWh</span>
        </div>
      {/if}

      <!-- Tertiary Metrics: Voltage, Current, Frequency -->
      <div class="tertiary-group">
        <div class="reading tertiary">
          <span class="reading-label">Voltage</span>
          <span class="reading-value">{latestReading.voltage.toFixed(1)} V</span>
        </div>
        <div class="reading tertiary">
          <span class="reading-label">Current</span>
          <span class="reading-value">{latestReading.current.toFixed(2)} A</span>
        </div>
        <div class="reading tertiary">
          <span class="reading-label">Freq</span>
          <span class="reading-value">{latestReading.ac_frequency.toFixed(1)} Hz</span>
        </div>
      </div>
    {:else}
      <div class="no-reading">No data</div>
    {/if}
  </div>
</Card>

{#if showColorPicker}
  <ColorPickerModal
    currentColor={sensor.color}
    onSelect={handleColorSelect}
    onClose={() => (showColorPicker = false)}
  />
{/if}

<style>
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

  .energy-header {
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

  .energy-name {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--color-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time-ago {
    font-size: 0.625rem;
    color: var(--color-text-subtle);
  }

  .energy-readings {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .reading {
    background: var(--color-surface-muted);
    padding: 0.5rem 0.625rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .reading.primary {
    background: linear-gradient(135deg, #fef3c7 0%, #fde68a 100%);
  }

  .reading.secondary {
    background: var(--color-surface-soft);
  }

  .tertiary-group {
    display: flex;
    gap: 0.5rem;
  }

  .reading.tertiary {
    flex: 1;
    background: var(--color-surface-muted);
    padding: 0.375rem 0.5rem;
  }

  .reading-label {
    font-size: 0.6875rem;
    color: var(--color-text-muted);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .reading-value {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
  }

  .reading.primary .reading-value {
    font-size: 1.25rem;
    color: #92400e;
  }

  .reading.tertiary .reading-value {
    font-size: 0.875rem;
  }

  .no-reading {
    text-align: center;
    color: var(--color-text-subtle);
    font-size: 0.875rem;
    padding: 0.5rem;
  }
</style>
