<script lang="ts">
  import UnifiedGraphPanel from "../lib/graphs/UnifiedGraphPanel.svelte";
  import SensorListPanel from "../lib/devices/SensorListPanel.svelte";
  import PageState from "../lib/shared/PageState.svelte";
  import type { DeviceInfo } from "../lib/api/devices";
  import { deviceStateMemory, sensorsMemory } from "../lib/memory";
  import { graphConfig } from "../lib/stores/graphConfig";
  import { onMount } from "svelte";

  let loading = $state(true);
  let error = $state<string | null>(null);
  let initialized = $state(false);

  // Filter: exclude energy meters (they belong in ConsommationsView)
  const isEnergyMeter = (device: DeviceInfo) =>
    device.capabilities.some(
      (cap) => cap.type === "sensor" && cap.sensor_type === "energy_meter"
    );

  // Initialize graphConfig with the correct sensors for this view
  function initializeGraphConfig(devices: DeviceInfo[]) {
    const sensorDevices = devices
      .filter((d) => !isEnergyMeter(d))
      .map((s) => ({ deviceId: s.device_id, color: s.color }));
    graphConfig.initializeSensors(sensorDevices);
  }

  async function loadData(force = false) {
    const sensorState = $sensorsMemory;

    if (!force && sensorState.devicesLoaded) {
      initializeGraphConfig(sensorState.devices);
      if (!initialized) {
        graphConfig.hideAll();
        initialized = true;
      }
      loading = false;
      return;
    }

    loading = true;
    error = null;

    try {
      const sensors = await sensorsMemory.ensureDevices(force);

      // Always initialize graphConfig for this view
      initializeGraphConfig(sensors);
      if (!initialized) {
        graphConfig.hideAll();
        initialized = true;
      }
      void deviceStateMemory.ensureDeviceStates(force).catch((err) => {
        console.error("Failed to fetch device states:", err);
      });
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load sensor data";
    } finally {
      loading = false;
    }
  }

  function retryLoad() {
    void loadData(true);
  }

  onMount(() => {
    void loadData();
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
  }

  .sensors-section {
    flex: 1;
    min-height: 0;
  }

  @media (max-width: 768px) {
    .unified-layout {
      gap: var(--section-gap, 1.5rem);
    }
  }

  @media (max-width: 480px) {
    .unified-layout {
      gap: var(--section-gap, 1rem);
    }
  }
</style>
