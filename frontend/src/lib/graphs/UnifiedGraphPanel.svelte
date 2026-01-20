<script lang="ts">
  import GraphToolbar from "./GraphToolbar.svelte";
  import UnifiedChart from "./UnifiedChart.svelte";
  import { graphConfig } from "../stores/graphConfig";

  let {
    metricType = "temperature"
  }: {
    metricType?: "temperature" | "power";
  } = $props();

  let sensors = $derived($graphConfig.sensors);
  let metric = $derived($graphConfig.metric);
  let timeRange = $derived($graphConfig.timeRange);
  let someHidden = $derived(sensors.some((s) => !s.visible));
</script>

<div class="graph-panel">
  <div class="panel-header">
    <GraphToolbar {someHidden} {metricType} />
  </div>

  <div class="chart-wrapper">
    <UnifiedChart {sensors} {metric} {timeRange} {metricType} />
  </div>
</div>

<style>
  .graph-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
  }

  .panel-header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .chart-wrapper {
    flex: 1;
    min-height: 400px;
    max-height: 800px;
  }

  @media (max-width: 768px) {
    .chart-wrapper {
      min-height: var(--graph-min-height, 300px);
      max-height: 50vh;
    }
  }

  @media (max-width: 480px) {
    .graph-panel {
      gap: 0.75rem;
    }

    .chart-wrapper {
      min-height: var(--graph-min-height, 250px);
      max-height: 45vh;
    }
  }
</style>
