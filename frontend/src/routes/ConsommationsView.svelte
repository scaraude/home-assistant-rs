<script lang="ts">
  import UnifiedGraphPanel from "../lib/UnifiedGraphPanel.svelte";
  import EnergyListPanel from "../lib/EnergyListPanel.svelte";
  import PageState from "../lib/PageState.svelte";
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

      // Initialize only energy meters in graph config
      const energyMeterIds = sensors
        .filter(s => s.capability_subtype === "energy_meter")
        .map(s => s.device_id);

      if ($graphConfig.sensors.length === 0 ||
          $graphConfig.sensors.length !== energyMeterIds.length) {
        graphConfig.initializeSensors(energyMeterIds);
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
      error = err instanceof Error ? err.message : "Failed to load energy data";
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

<div class="consommations-view">
  <PageState {loading} {error} loadingText="Loading energy meters..." onRetry={retryLoad}>
    <div class="unified-layout">
      <div class="graph-section">
        <UnifiedGraphPanel metricType="power" />
      </div>
      <div class="meters-section">
        <EnergyListPanel />
      </div>
    </div>
  </PageState>
</div>

<style>
  .consommations-view {
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

  .meters-section {
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
