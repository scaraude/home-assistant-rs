<script lang="ts">
  import CompactSensorCard from "./CompactSensorCard.svelte";
  import { graphConfig } from "./stores/graphConfig";
  import { dataCache } from "./stores/dataCache";
  import type { SensorReading } from "./api";

  let sensors = $derived($graphConfig.sensors);
  let readings = $derived($dataCache.sensors.readings);

  // Get latest reading for each sensor
  function getLatestReading(deviceId: string): SensorReading | null {
    const sensorReadings = readings
      .filter(r => r.device_id === deviceId)
      .sort((a, b) => b.timestamp - a.timestamp);

    return sensorReadings[0] || null;
  }
</script>

<div class="sensor-list-panel">
  <h3>Sensors</h3>

  {#if sensors.length === 0}
    <div class="no-sensors">
      <p>No sensors found</p>
    </div>
  {:else}
    <div class="sensor-grid">
      {#each sensors as sensor (sensor.deviceId)}
        <CompactSensorCard
          {sensor}
          latestReading={getLatestReading(sensor.deviceId)}
        />
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

  h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
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
      gap: 0.75rem;
    }

    h3 {
      font-size: 1rem;
    }
  }
</style>
