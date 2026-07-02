<script lang="ts">
  import { graphConfig } from "../stores/graphConfig";
  import type { GraphState } from "../stores/graphConfig";
  import { hourCycle, useHour12 } from "../utils/time";

  let {
    metricType = "temperature"
  }: {
    metricType?: "temperature" | "power";
  } = $props();

  const currentMetric = $derived($graphConfig.metric);
  const currentTimeRange = $derived($graphConfig.timeRange);
  const clock12 = $derived(
    $hourCycle === "24h" ? false : $hourCycle === "12h" ? true : useHour12(),
  );

  function setMetric(metric: GraphState['metric']) {
    graphConfig.setMetric(metric);
  }

  function setTimeRange(timeRange: GraphState['timeRange']) {
    graphConfig.setTimeRange(timeRange);
  }
</script>

<div class="toolbar">
  {#if metricType === "temperature"}
    <div class="toolbar-section">
      <span class="section-label">Metric:</span>
      <div class="button-group">
        <button
          class="toolbar-btn"
          class:active={currentMetric === 'temperature'}
          onclick={() => setMetric('temperature')}
        >
          Temp
        </button>
        <button
          class="toolbar-btn"
          class:active={currentMetric === 'humidity'}
          onclick={() => setMetric('humidity')}
        >
          Humidity
        </button>
        <button
          class="toolbar-btn"
          class:active={currentMetric === 'both'}
          onclick={() => setMetric('both')}
        >
          Both
        </button>
      </div>
    </div>

    <div class="toolbar-divider"></div>
  {/if}

  <div class="toolbar-section">
    <span class="section-label">Range:</span>
    <div class="button-group">
      <button
        class="toolbar-btn"
        class:active={currentTimeRange === '24h'}
        onclick={() => setTimeRange('24h')}
      >
        24h
      </button>
      <button
        class="toolbar-btn"
        class:active={currentTimeRange === '1w'}
        onclick={() => setTimeRange('1w')}
      >
        1w
      </button>
      <button
        class="toolbar-btn"
        class:active={currentTimeRange === '1m'}
        onclick={() => setTimeRange('1m')}
      >
        1m
      </button>
      <button
        class="toolbar-btn"
        class:active={currentTimeRange === '1y'}
        onclick={() => setTimeRange('1y')}
      >
        1y
      </button>
    </div>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-section">
    <span class="section-label">Clock:</span>
    <div class="button-group">
      <button
        class="toolbar-btn"
        class:active={!clock12}
        onclick={() => hourCycle.set('24h')}
      >
        24h
      </button>
      <button
        class="toolbar-btn"
        class:active={clock12}
        onclick={() => hourCycle.set('12h')}
      >
        12h
      </button>
    </div>
  </div>

</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: #f9fafb;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
    flex-wrap: wrap;
  }

  .toolbar-section {
    display: flex;
    align-items: center;
    gap: 0.625rem;
  }

  .section-label {
    font-size: 0.8125rem;
    color: #6b7280;
    font-weight: 500;
  }

  .button-group {
    display: flex;
    gap: 0.375rem;
    background: white;
    padding: 0.25rem;
    border-radius: 6px;
    border: 1px solid #d1d5db;
  }

  .toolbar-btn {
    padding: 0.375rem 0.75rem;
    background: transparent;
    border: none;
    border-radius: 4px;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .toolbar-btn:hover {
    background: #f3f4f6;
    color: #111827;
  }

  .toolbar-btn.active {
    background: #3b82f6;
    color: white;
  }

  .toolbar-divider {
    width: 1px;
    height: 24px;
    background: #d1d5db;
  }

  @media (max-width: 640px) {
    .toolbar {
      flex-direction: column;
      align-items: stretch;
      gap: 0.75rem;
    }

    .toolbar-section {
      justify-content: space-between;
    }

    .toolbar-divider {
      display: none;
    }

    .button-group {
      flex: 1;
    }

    .toolbar-btn {
      flex: 1;
      text-align: center;
    }

  }
</style>
