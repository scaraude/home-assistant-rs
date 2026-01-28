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
</script>

<div class="graph-panel">
  <div class="panel-header">
    <GraphToolbar {metricType} />
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
  }

  .panel-header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .chart-wrapper {
    flex: 0 0 auto;
    height: clamp(var(--graph-min-height, 300px), 60vh, 800px);
  }

  @media (max-width: 768px) {
    .chart-wrapper {
      height: clamp(var(--graph-min-height, 300px), 50vh, 560px);
    }
  }

  @media (max-width: 480px) {
    .graph-panel {
      gap: 0.75rem;
    }

    .chart-wrapper {
      height: clamp(var(--graph-min-height, 250px), 45vh, 360px);
    }
  }
</style>
