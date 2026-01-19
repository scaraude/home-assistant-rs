<script lang="ts">
  import EnergyMeterCard from "./EnergyMeterCard.svelte";
  import { dataCache } from "../stores/dataCache";
  import { graphConfig } from "../stores/graphConfig";
  import type { EnergySensorReading } from "../api";
  import type { DeviceInfo } from "../api/devices";

  let editMode = $state(false);

  function toggleEditMode() {
    editMode = !editMode;
  }

  const isEnergyMeter = (device: DeviceInfo | undefined) =>
    device?.capabilities.some(
      (cap) => cap.type === "sensor" && cap.sensor_type === "energy_meter"
    );

  let energyMeters = $derived(
    $graphConfig.sensors.filter(s => {
      const device = $dataCache.sensors.devices.find(d => d.device_id === s.deviceId);
      return isEnergyMeter(device);
    })
  );

  function getLatestReading(deviceId: string): EnergySensorReading | null {
    const deviceReadings = $dataCache.sensors.readings
      .filter(r => r.device_id === deviceId)
      .sort((a, b) => b.timestamp - a.timestamp);

    if (deviceReadings.length === 0) return null;

    const latestReading = deviceReadings[0];
    return latestReading.type === "energy_meter" ? latestReading : null;
  }
</script>

<div class="energy-list-panel">
  <div class="panel-header">
    <h2>Energy Meters</h2>
    <button class="edit-btn" onclick={toggleEditMode}>
      {editMode ? "Done" : "Edit"}
    </button>
  </div>

  <div class="energy-grid">
    {#each energyMeters as meter (meter.deviceId)}
      <EnergyMeterCard
        sensor={meter}
        latestReading={getLatestReading(meter.deviceId)}
        {editMode}
      />
    {/each}
  </div>

  {#if energyMeters.length === 0}
    <div class="no-meters">
      <p>No energy meters found</p>
      <p class="help-text">Energy meters will appear here once detected</p>
    </div>
  {/if}
</div>

<style>
  .energy-list-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 0.5rem;
  }

  h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: #111827;
    margin: 0;
  }

  .edit-btn {
    padding: 0.5rem 1rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
    cursor: pointer;
    transition: all 0.15s;
  }

  .edit-btn:hover {
    background: #e5e7eb;
    border-color: #9ca3af;
  }

  .energy-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .no-meters {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    text-align: center;
    color: #6b7280;
  }

  .no-meters p {
    margin: 0.25rem 0;
  }

  .help-text {
    font-size: 0.875rem;
    color: #9ca3af;
  }

  @media (max-width: 768px) {
    .energy-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
