<script lang="ts">
  import CompactSensorCard from "./CompactSensorCard.svelte";
  import PresenceCard from "./PresenceCard.svelte";
  import { graphConfig } from "../stores/graphConfig";
  import { dataCache } from "../stores/dataCache";
  import type { SensorReading } from "../api";
  import type { DeviceInfo } from "../api/devices";

  const allSensors = $derived($graphConfig.sensors);
  const devices = $derived($dataCache.sensors.devices);
  const readings = $derived($dataCache.sensors.readings);

  // Filter out energy meters - they belong in ConsommationsView
  const isEnergyMeter = (device: DeviceInfo) =>
    device.capabilities.some(
      (cap) => cap.type === "sensor" && cap.sensor_type === "energy_meter"
    );

  const deviceMap = $derived(new Map(devices.map(d => [d.device_id, d])));

  // Only show non-energy-meter sensors
  const sensors = $derived(
    allSensors.filter(s => {
      const device = deviceMap.get(s.deviceId);
      return device && !isEnergyMeter(device);
    })
  );

  let editMode = $state(false);

  // Get latest reading for each sensor
  function getLatestReading(deviceId: string): SensorReading | null {
    const sensorReadings = readings
      .filter(r => r.device_id === deviceId)
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());

    return sensorReadings[0] || null;
  }

  // Check if sensor is a presence sensor
  function isPresenceSensor(deviceId: string): boolean {
    const reading = getLatestReading(deviceId);
    return reading?.type === 'presence';
  }

  // Check if any sensors are selected
  const hasSelectedSensors = $derived(sensors.some(s => s.visible));

  // Handle select/unselect all toggle
  const handleToggleAll = () => {
    if (hasSelectedSensors) {
      // Unselect all: hide all visible sensors
      sensors.forEach(sensor => {
        if (sensor.visible) {
          graphConfig.toggleSensor(sensor.deviceId);
        }
      });
    } else {
      // Select all: show all sensors
      graphConfig.showAll();
    }
  };

  // Toggle edit mode
  const toggleEditMode = () => {
    editMode = !editMode;
  };
</script>

<div class="sensor-list-panel">
  <div class="header">
    <h3>Sensors</h3>
    <div class="header-actions">
      <button
        class="action-button ghost"
        onclick={handleToggleAll}
        title={hasSelectedSensors ? "Unselect all sensors" : "Select all sensors"}
        aria-label={hasSelectedSensors ? "Unselect all sensors" : "Select all sensors"}
      >
        {#if hasSelectedSensors}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
          Unselect All
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          Select All
        {/if}
      </button>

      <button
        class="icon-button"
        class:active={editMode}
        onclick={toggleEditMode}
        title={editMode ? "Close edit mode" : "Edit sensor names"}
        aria-label={editMode ? "Close edit mode" : "Edit sensor names"}
      >
        {#if editMode}
          <!-- X icon (close) -->
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        {:else}
          <!-- Edit icon -->
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
          </svg>
        {/if}
      </button>
    </div>
  </div>

  {#if sensors.length === 0}
    <div class="no-sensors">
      <p>No sensors found</p>
    </div>
  {:else}
    <div class="sensor-grid">
      {#each sensors as sensor (sensor.deviceId)}
        {@const reading = getLatestReading(sensor.deviceId)}
        {#if isPresenceSensor(sensor.deviceId)}
          <PresenceCard
            {sensor}
            {editMode}
            latestReading={reading?.type === 'presence' ? reading : null}
          />
        {:else}
          <CompactSensorCard
            {sensor}
            {editMode}
            latestReading={reading}
          />
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .sensor-list-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .action-button {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.875rem;
    background: #ffffff;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    cursor: pointer;
    color: #374151;
    font-size: 0.875rem;
    font-weight: 500;
    transition: all 0.2s;
  }

  .action-button.ghost {
    background: transparent;
    border-color: transparent;
    color: #6b7280;
  }

  .action-button:hover {
    background: #f9fafb;
    border-color: #9ca3af;
    color: #111827;
  }

  .action-button.ghost:hover {
    background: #f3f4f6;
    border-color: transparent;
    color: #374151;
  }

  .action-button:active {
    background: #f3f4f6;
  }

  .action-button:focus-visible {
    outline: 2px solid #3b82f6;
    outline-offset: 2px;
  }

  .action-button svg {
    flex-shrink: 0;
  }

  .icon-button {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem;
    background: transparent;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    cursor: pointer;
    color: #6b7280;
    transition: all 0.2s;
  }

  .icon-button:hover {
    background: #f3f4f6;
    border-color: #9ca3af;
    color: #374151;
  }

  .icon-button.active {
    background: #3b82f6;
    border-color: #3b82f6;
    color: #ffffff;
  }

  .icon-button.active:hover {
    background: #2563eb;
    border-color: #2563eb;
  }

  .icon-button:focus-visible {
    outline: 2px solid #3b82f6;
    outline-offset: 2px;
  }

  .no-sensors {
    text-align: center;
    padding: 3rem 0;
    color: #6b7280;
  }

  .sensor-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  /* Responsive breakpoints */
  @media (min-width: 1920px) {
    .sensor-grid {
      grid-template-columns: repeat(6, 1fr);
    }
  }

  @media (min-width: 1440px) and (max-width: 1919px) {
    .sensor-grid {
      grid-template-columns: repeat(5, 1fr);
    }
  }

  @media (min-width: 1024px) and (max-width: 1439px) {
    .sensor-grid {
      grid-template-columns: repeat(4, 1fr);
    }
  }

  @media (min-width: 768px) and (max-width: 1023px) {
    .sensor-grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }

  @media (max-width: 767px) {
    .sensor-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: var(--card-gap, 0.75rem);
    }

    .header {
      flex-wrap: wrap;
    }

    h3 {
      font-size: 1rem;
    }

    .action-button {
      font-size: 0.8125rem;
      padding: 0.4rem 0.75rem;
    }

    .icon-button {
      padding: 0.4rem;
    }

    .icon-button svg {
      width: 16px;
      height: 16px;
    }
  }

  /* Small phones - single column */
  @media (max-width: 480px) {
    .sensor-grid {
      grid-template-columns: 1fr;
      gap: var(--card-gap, 0.5rem);
    }

    .header {
      gap: 0.5rem;
    }

    .header-actions {
      width: 100%;
      justify-content: flex-end;
    }

    .action-button {
      flex: 1;
      justify-content: center;
    }
  }
</style>
