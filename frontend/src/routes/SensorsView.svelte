<script lang="ts">
  import UnifiedGraphPanel from "../lib/graphs/UnifiedGraphPanel.svelte";
  import SensorListPanel from "../lib/devices/SensorListPanel.svelte";
  import PageState from "../lib/shared/PageState.svelte";
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
        readingsResult.latestTimestamp ?? null,
        hours
      );

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

  onMount(() => {
    const currentHours = TIME_RANGE_HOURS[$graphConfig.timeRange];
    void loadData(currentHours);
  });

  let previousTimeRange: string | undefined = $state(undefined);
  $effect.pre(() => {
    const currentTimeRange = $graphConfig.timeRange;
    if (previousTimeRange !== undefined && currentTimeRange !== previousTimeRange) {
      const currentHours = TIME_RANGE_HOURS[currentTimeRange];
      void loadData(currentHours, true);
    }
    previousTimeRange = currentTimeRange;
  });
</script>

<div class="sensor-view">
  <PageState {loading} {error} loadingText="Loading sensors..." onRetry={retryLoad}>
    <div class="unified-layout">
      <div class="graph-section">
        <UnifiedGraphPanel />
      </div>
      <div class="sensors-section">
        <SensorListPanel />
      </div>
    </div>
  </PageState>
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
