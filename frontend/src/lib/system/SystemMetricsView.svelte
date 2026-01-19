<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    Chart,
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    TimeScale,
    Title,
    Tooltip,
    Legend,
  } from 'chart.js';
  import zoomPlugin from 'chartjs-plugin-zoom';
  import 'chartjs-adapter-date-fns';
  import type { SystemMonitorEntry } from "../api";

  Chart.register(
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    TimeScale,
    Title,
    Tooltip,
    Legend,
    zoomPlugin
  );

  let {
    entries = []
  }: {
    entries?: SystemMonitorEntry[];
  } = $props();

  let combinedCanvas = $state<HTMLCanvasElement>();
  let combinedChart = $state<Chart | null>(null);
  let previousEntriesLength = $state(0);

  function parseTimestamps() {
    return entries.map((e) => new Date(e.timestamp).getTime());
  }

  function createCombinedChart() {
    if (!combinedCanvas || entries.length === 0) return;

    const ctx = combinedCanvas.getContext('2d');
    if (!ctx) return;

    const timestamps = parseTimestamps();
    const cpuData = entries.map((e) => e.cpu_usage);
    const ramData = entries.map((e) => e.ram_usage);
    const tempData = entries.map((e) => e.cpu_temp);

    combinedChart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets: [
          {
            label: 'CPU Usage (%)',
            data: cpuData,
            borderColor: 'rgb(59, 130, 246)',
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
            yAxisID: 'y',
            tension: 0.4,
            pointRadius: 0.3,
            pointHoverRadius: 4,
            fill: false,
          },
          {
            label: 'RAM Usage (%)',
            data: ramData,
            borderColor: 'rgb(34, 197, 94)',
            backgroundColor: 'rgba(34, 197, 94, 0.1)',
            yAxisID: 'y',
            tension: 0.4,
            pointRadius: 0.3,
            pointHoverRadius: 4,
            fill: false,
          },
          {
            label: 'CPU Temp (°C)',
            data: tempData,
            borderColor: 'rgb(239, 68, 68)',
            backgroundColor: 'rgba(239, 68, 68, 0.1)',
            yAxisID: 'y1',
            tension: 0.4,
            pointRadius: 0.3,
            pointHoverRadius: 4,
            fill: false,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
          mode: 'index',
          intersect: false,
        },
        plugins: {
          legend: {
            display: true,
            position: 'top',
            labels: {
              usePointStyle: true,
              padding: 15,
              font: { size: 12 },
            },
          },
          tooltip: {
            enabled: true,
            callbacks: {
              title: (items) => {
                if (items.length > 0 && items[0].parsed.x !== null) {
                  return new Date(items[0].parsed.x).toLocaleString();
                }
                return '';
              },
            },
          },
          zoom: {
            zoom: {
              wheel: {
                enabled: true,
              },
              pinch: {
                enabled: true,
              },
              mode: 'x',
            },
            pan: {
              enabled: true,
              mode: 'x',
            },
            limits: {
              x: {
                min: timestamps[0],
                max: timestamps[timestamps.length - 1],
                minRange: 60 * 1000, // Minimum 1 minute zoom range
              },
            },
          },
        },
        scales: {
          x: {
            type: 'time',
            time: {
              tooltipFormat: 'MMM d, HH:mm',
              displayFormats: { hour: 'HH:mm', day: 'MMM d' },
            },
            grid: { display: false },
            ticks: { maxRotation: 0, font: { size: 10 } },
          },
          y: {
            type: 'linear',
            display: true,
            position: 'left',
            min: 0,
            max: 100,
            title: {
              display: true,
              text: 'Usage (%)',
              font: { size: 11 },
            },
            ticks: { font: { size: 10 } },
            grid: { color: 'rgba(0, 0, 0, 0.05)' },
          },
          y1: {
            type: 'linear',
            display: true,
            position: 'right',
            title: {
              display: true,
              text: 'Temperature (°C)',
              font: { size: 11 },
            },
            ticks: { font: { size: 10 } },
            grid: { drawOnChartArea: false },
          },
        },
      },
    });
  }

  function updateChart() {
    if (!combinedChart || entries.length === 0) return;

    const timestamps = parseTimestamps();

    combinedChart.data.labels = timestamps;
    combinedChart.data.datasets[0].data = entries.map((e) => e.cpu_usage);
    combinedChart.data.datasets[1].data = entries.map((e) => e.ram_usage);
    combinedChart.data.datasets[2].data = entries.map((e) => e.cpu_temp);

    // Update zoom limits to match new data range
    if (combinedChart.options.plugins?.zoom?.limits?.x) {
      combinedChart.options.plugins.zoom.limits.x.min = timestamps[0];
      combinedChart.options.plugins.zoom.limits.x.max = timestamps[timestamps.length - 1];
    }

    combinedChart.update('none');
  }

  function handleCanvasDoubleClick() {
    if (combinedChart) {
      combinedChart.resetZoom();
    }
  }

  onMount(() => {
    if (entries.length > 0) {
      createCombinedChart();
    }
  });

  onDestroy(() => {
    if (combinedChart) combinedChart.destroy();
  });

  $effect(() => {
    if (entries && entries.length > 0) {
      // If data length changed significantly (>20%), recreate chart to reset zoom
      const lengthChangeRatio = Math.abs(entries.length - previousEntriesLength) / Math.max(previousEntriesLength, 1);
      const significantChange = lengthChangeRatio > 0.2;

      if (!combinedChart) {
        createCombinedChart();
        previousEntriesLength = entries.length;
      } else if (significantChange) {
        // Destroy and recreate chart on significant data changes (time range change)
        combinedChart.destroy();
        combinedChart = null;
        createCombinedChart();
        previousEntriesLength = entries.length;
      } else {
        updateChart();
        previousEntriesLength = entries.length;
      }
    }
  });

  let latestEntry = $derived(entries.length > 0 ? entries[entries.length - 1] : null);
</script>

<div class="system-metrics">
  {#if latestEntry}
    <div class="stats-row">
      <div class="stat-card">
        <div class="stat-label">CPU Usage</div>
        <div class="stat-value">{latestEntry.cpu_usage.toFixed(1)}%</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">RAM Usage</div>
        <div class="stat-value">{latestEntry.ram_usage.toFixed(1)}%</div>
        <div class="stat-detail">
          {latestEntry.ram_used.toFixed(0)} / {latestEntry.ram_total.toFixed(0)} MB
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-label">CPU Temperature</div>
        <div class="stat-value">{latestEntry.cpu_temp.toFixed(1)}°C</div>
      </div>
    </div>
  {/if}

  <div class="chart-card">
    <h3>System Metrics</h3>
    <div class="zoom-hint">Scroll to zoom • Drag to pan • Double-click to reset</div>
    <div class="graph-container-large">
      <canvas bind:this={combinedCanvas} ondblclick={handleCanvasDoubleClick}></canvas>
    </div>
  </div>
</div>

<style>
  .system-metrics {
    width: 100%;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .stat-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
    text-align: center;
  }

  .stat-label {
    font-size: 0.875rem;
    color: #6b7280;
    margin-bottom: 0.5rem;
  }

  .stat-value {
    font-size: 1.875rem;
    font-weight: 600;
    color: #111827;
  }

  .stat-detail {
    font-size: 0.75rem;
    color: #9ca3af;
    margin-top: 0.25rem;
  }

  .chart-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }

  .chart-card h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
  }

  .zoom-hint {
    font-size: 0.75rem;
    color: #6b7280;
    margin-bottom: 1rem;
    font-style: italic;
  }

  .graph-container-large {
    width: 100%;
    height: 400px;
    position: relative;
  }

  canvas {
    max-width: 100%;
    max-height: 100%;
  }
</style>
