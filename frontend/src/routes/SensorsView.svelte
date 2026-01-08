<script lang="ts">
  import UnifiedGraphPanel from "../lib/UnifiedGraphPanel.svelte";
  import SensorListPanel from "../lib/SensorListPanel.svelte";
  import { fetchSensors, fetchReadings, fetchDeviceStates } from "../lib/api";
  import { dataCache } from "../lib/stores/dataCache";
  import { graphConfig, TIME_RANGE_HOURS } from "../lib/stores/graphConfig";
  import { onMount } from "svelte";

  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadData(hours: number, force = false) {
    const sensorState = $dataCache.sensors;

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

      // Initialize graphConfig sensors if not already done
      const sensorIds = sensors.map(s => s.device_id);
      if ($graphConfig.sensors.length === 0 ||
          $graphConfig.sensors.length !== sensorIds.length) {
        graphConfig.initializeSensors(sensorIds);
      }

      try {
        const states = await fetchDeviceStates();
        states.forEach((state) => {
          dataCache.updateDeviceState(state.device_id, state);
        });
      } catch (err) {
        console.error("Failed to fetch device states:", err);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load sensor data";
    } finally {
      loading = false;
    }
  }

  function retryLoad() {
    const currentHours = TIME_RANGE_HOURS[$graphConfig.timeRange];
    void loadData(currentHours, true);
  }

  // Load data on mount and when time range changes
  onMount(() => {
    const currentHours = TIME_RANGE_HOURS[$graphConfig.timeRange];
    void loadData(currentHours);
  });

  // Watch for time range changes using $effect.pre for tracking
  let previousTimeRange: string | undefined = $state(undefined);
  $effect.pre(() => {
    const currentTimeRange = $graphConfig.timeRange;

    // Skip the initial run when previousTimeRange is undefined
    if (previousTimeRange !== undefined && currentTimeRange !== previousTimeRange) {
      const currentHours = TIME_RANGE_HOURS[currentTimeRange];
      void loadData(currentHours, true);
    }

    previousTimeRange = currentTimeRange;
  });
</script>

<div class="sensor-view">
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
  {:else}
    <div class="unified-layout">
      <div class="graph-section">
        <UnifiedGraphPanel />
      </div>
      <div class="sensors-section">
        <SensorListPanel />
      </div>
    </div>
  {/if}
</div>

<style>
  .sensor-view {
    width: 100%;
    height: 100%;
  }

  .unified-layout {
    display: flex;
    flex-direction: column;
    gap: 2rem;
    height: 100%;
  }

  .graph-section {
    flex: 0 0 auto;
    height: 60vh;
    min-height: 400px;
    max-height: 800px;
  }

  .sensors-section {
    flex: 1;
    min-height: 0;
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

  @media (max-width: 768px) {
    .graph-section {
      height: 50vh;
      min-height: 300px;
    }

    .unified-layout {
      gap: 1.5rem;
    }
  }
</style>
