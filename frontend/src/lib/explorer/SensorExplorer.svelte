<script lang="ts">
  import { onMount } from "svelte";
  import CategoryRail from "./CategoryRail.svelte";
  import SelectedSeriesBar from "./SelectedSeriesBar.svelte";
  import MultiMetricChart from "./MultiMetricChart.svelte";
  import { explorerConfig, type ExplorerTimeRange } from "./explorerStore";
  import { sensorsMemory } from "../memory";
  import { hourCycle, useHour12 } from "../utils/time";

  const series = $derived($explorerConfig.series);
  const timeRange = $derived($explorerConfig.timeRange);
  const clock12 = $derived(
    $hourCycle === "24h" ? false : $hourCycle === "12h" ? true : useHour12(),
  );

  const RANGES: { id: ExplorerTimeRange; label: string }[] = [
    { id: "24h", label: "24h" },
    { id: "1w", label: "1s" },
    { id: "1m", label: "1m" },
    { id: "1y", label: "1a" },
  ];

  onMount(() => {
    // Populate "latest value" labels for every sensor in the rail.
    void sensorsMemory.refreshLatestReadings().catch((e) => console.error(e));
  });
</script>

<div class="board">
  <aside class="rail">
    <CategoryRail />
  </aside>

  <section class="graph-card">
    <header class="graph-toolbar">
      <div class="segmented">
        {#each RANGES as r (r.id)}
          <button
            class="seg"
            class:active={timeRange === r.id}
            onclick={() => explorerConfig.setTimeRange(r.id)}
          >
            {r.label}
          </button>
        {/each}
      </div>

      <div class="segmented">
        <button class="seg" class:active={!clock12} onclick={() => hourCycle.set("24h")}>24h</button>
        <button class="seg" class:active={clock12} onclick={() => hourCycle.set("12h")}>12h</button>
      </div>

      <span class="hint">Glisser pour zoomer · Maj+glisser pour naviguer · clic droit pour réinitialiser</span>
    </header>

    {#if series.length > 0}
      <div class="chips">
        <SelectedSeriesBar {series} />
      </div>
    {/if}

    <div class="chart">
      <MultiMetricChart {series} {timeRange} />
    </div>
  </section>
</div>

<style>
  /* The gray "board" ties the accordion and the graph into one integrated
     surface; both sit on it as white panels with a tight gutter. */
  .board {
    display: grid;
    grid-template-columns: minmax(240px, 288px) 1fr;
    gap: 10px;
    padding: 1.25rem;
    background: #eef1f5;
    border-radius: 18px;
    margin: 1.25rem;
    align-items: stretch;
    min-height: calc(100vh - var(--app-header-height, 88px) - 2.5rem);
  }

  .rail {
    min-height: 0;
  }

  .graph-card {
    display: flex;
    flex-direction: column;
    background: white;
    border-radius: 12px;
    box-shadow:
      0 1px 2px rgba(16, 24, 40, 0.04),
      0 1px 3px rgba(16, 24, 40, 0.06);
    min-width: 0;
    overflow: hidden;
  }

  .graph-toolbar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f1f3f5;
    flex-wrap: wrap;
  }

  .segmented {
    display: inline-flex;
    background: #f2f4f7;
    border-radius: 8px;
    padding: 3px;
    gap: 2px;
  }

  .seg {
    padding: 0.3rem 0.7rem;
    border: none;
    background: transparent;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #667085;
    cursor: pointer;
    transition: all 0.15s;
  }

  .seg:hover {
    color: #101828;
  }

  .seg.active {
    background: white;
    color: #101828;
    box-shadow: 0 1px 2px rgba(16, 24, 40, 0.12);
  }

  .hint {
    margin-left: auto;
    font-size: 0.75rem;
    color: #b0b7c3;
  }

  .chips {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f1f3f5;
  }

  .chart {
    flex: 1;
    min-height: 0;
    padding: 0.75rem;
  }

  @media (max-width: 768px) {
    .board {
      grid-template-columns: 1fr;
      padding: 0.75rem;
      margin: 0.75rem;
      gap: 0.75rem;
    }

    .hint {
      display: none;
    }

    .chart {
      height: clamp(300px, 50vh, 560px);
    }
  }
</style>
