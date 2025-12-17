<script lang="ts">
  import { onMount } from "svelte";
  import SensorCard from "../lib/SensorCard.svelte";
  import {
    fetchSensors,
    fetchReadings,
    type SensorData,
    type SensorReading,
    type DeviceInfo,
  } from "../lib/api";
  import { dataCache } from "../lib/stores/dataCache";
  import { get } from "svelte/store";

  type TimeRange = "1h" | "6h" | "24h" | "week";

  const TIME_RANGE_KEY = "homeAssistant:sensorTimeRange";

  // Map time ranges to hours
  const timeRangeToHours: Record<TimeRange, number> = {
    "1h": 1,
    "6h": 6,
    "24h": 24,
    week: 168, // 7 days
  };

  let selectedTimeRange = $state<TimeRange>("24h");
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Build sensor data from readings
  function buildSensorData(
    sensors: DeviceInfo[],
    readings: SensorReading[]
  ): SensorData[] {
    return sensors.map(({ device_id, name }) => {
      const sensorReadings = readings
        .filter((r) => r.device_id === device_id)
        .sort((a, b) => a.timestamp - b.timestamp);

      return {
        device_id,
        name,
        latestReading: sensorReadings[sensorReadings.length - 1] || null,
        history: sensorReadings,
      };
    });
  }

  async function loadData(range: TimeRange, force = false) {
    const hours = timeRangeToHours[range];
    const sensorState = get(dataCache).sensors;

    if (!force && sensorState.loaded && sensorState.rangeHours === hours) {
      loading = false;
      return;
    }

    loading = true;
    error = null;

    try {
      const [sensors, readingsResult] = await Promise.all([
        fetchSensors(),
        fetchReadings(undefined, hours),
      ]);

      dataCache.setSensors(sensors);
      dataCache.setSensorReadings(
        readingsResult.readings,
        readingsResult.latestTimestamp ?? 0,
        hours
      );
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load sensor data";
    } finally {
      loading = false;
    }
  }

  function setTimeRange(range: TimeRange) {
    if (selectedTimeRange === range) {
      return;
    }
    selectedTimeRange = range;
    localStorage.setItem(TIME_RANGE_KEY, range);
    void loadData(range, true);
  }

  function retryLoad() {
    void loadData(selectedTimeRange, true);
  }

  onMount(() => {
    const savedTimeRange = localStorage.getItem(TIME_RANGE_KEY);
    if (
      savedTimeRange === "1h" ||
      savedTimeRange === "6h" ||
      savedTimeRange === "24h" ||
      savedTimeRange === "week"
    ) {
      selectedTimeRange = savedTimeRange;
    }

    void loadData(selectedTimeRange);
  });

  let sensorData = $derived(buildSensorData(
    $dataCache.sensors.devices,
    $dataCache.sensors.readings
  ));
</script>

<div class="sensor-view">
  <div class="sensor-header">
    <h2>Sensors</h2>
    <div class="time-range-selector">
      <span class="time-range-label">Time range:</span>
      <button
        class="time-range-btn"
        class:active={selectedTimeRange === "1h"}
        onclick={() => setTimeRange("1h")}
      >
        1h
      </button>
      <button
        class="time-range-btn"
        class:active={selectedTimeRange === "6h"}
        onclick={() => setTimeRange("6h")}
      >
        6h
      </button>
      <button
        class="time-range-btn"
        class:active={selectedTimeRange === "24h"}
        onclick={() => setTimeRange("24h")}
      >
        24h
      </button>
      <button
        class="time-range-btn"
        class:active={selectedTimeRange === "week"}
        onclick={() => setTimeRange("week")}
      >
        Week
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>Loading sensors...</p>
    </div>
  {:else if error}
    <div class="error">
      <p>Error: {error}</p>
      <button onclick={retryLoad}>Retry</button>
    </div>
  {:else if sensorData.length === 0}
    <div class="no-sensors">
      <p>No sensors found</p>
    </div>
  {:else}
    <div class="sensor-grid">
      {#each sensorData as sensor (sensor.name)}
        <SensorCard sensorData={sensor} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .sensor-view {
    width: 100%;
  }

  .sensor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .sensor-header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #111827;
  }

  .time-range-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background-color: #f9fafb;
    padding: 0.25rem;
    border-radius: 6px;
    border: 1px solid #e5e7eb;
  }

  .time-range-label {
    font-size: 0.8125rem;
    color: #6b7280;
    font-weight: 500;
    padding: 0 0.5rem;
  }

  .time-range-btn {
    padding: 0.375rem 0.75rem;
    background: transparent;
    border: none;
    border-radius: 4px;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.15s;
  }

  .time-range-btn:hover {
    background: #e5e7eb;
    color: #111827;
  }

  .time-range-btn.active {
    background: #3b82f6;
    color: white;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 0;
    color: #6b7280;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 0;
    color: #dc2626;
  }

  .error button {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background 0.2s;
  }

  .error button:hover {
    background: #2563eb;
  }

  .no-sensors {
    text-align: center;
    padding: 4rem 0;
    color: #6b7280;
  }

  .sensor-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.5rem;
  }

  @media (max-width: 640px) {
    .sensor-grid {
      grid-template-columns: 1fr;
    }

    .sensor-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .sensor-header h2 {
      font-size: 1.25rem;
    }

    .time-range-selector {
      width: 100%;
      justify-content: space-between;
    }

    .time-range-btn {
      flex: 1;
    }
  }
</style>
