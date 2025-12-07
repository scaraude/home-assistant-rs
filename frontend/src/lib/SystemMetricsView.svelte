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
  import 'chartjs-adapter-date-fns';
  import type { SystemMonitorEntry } from './api';

  Chart.register(
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    TimeScale,
    Title,
    Tooltip,
    Legend
  );

  export let entries: SystemMonitorEntry[] = [];

  let cpuCanvas: HTMLCanvasElement;
  let ramCanvas: HTMLCanvasElement;
  let tempCanvas: HTMLCanvasElement;

  let cpuChart: Chart | null = null;
  let ramChart: Chart | null = null;
  let tempChart: Chart | null = null;

  function parseTimestamps() {
    return entries.map((e) => new Date(e.timestamp).getTime());
  }

  function createCpuChart() {
    if (!cpuCanvas || entries.length === 0) return;

    const ctx = cpuCanvas.getContext('2d');
    if (!ctx) return;

    const timestamps = parseTimestamps();
    const cpuData = entries.map((e) => e.cpu_usage);

    cpuChart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets: [
          {
            label: 'CPU Usage (%)',
            data: cpuData,
            borderColor: 'rgb(59, 130, 246)',
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
            tension: 0.4,
            pointRadius: 0.3,
            pointHoverRadius: 4,
            fill: true,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
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
            min: 0,
            max: 100,
            ticks: { font: { size: 10 } },
            grid: { color: 'rgba(0, 0, 0, 0.05)' },
          },
        },
      },
    });
  }

  function createRamChart() {
    if (!ramCanvas || entries.length === 0) return;

    const ctx = ramCanvas.getContext('2d');
    if (!ctx) return;

    const timestamps = parseTimestamps();
    const ramData = entries.map((e) => e.ram_usage);

    ramChart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets: [
          {
            label: 'RAM Usage (%)',
            data: ramData,
            borderColor: 'rgb(34, 197, 94)',
            backgroundColor: 'rgba(34, 197, 94, 0.1)',
            tension: 0.4,
            pointRadius: 0.3,
            pointHoverRadius: 4,
            fill: true,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
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
            min: 0,
            max: 100,
            ticks: { font: { size: 10 } },
            grid: { color: 'rgba(0, 0, 0, 0.05)' },
          },
        },
      },
    });
  }

  function createTempChart() {
    if (!tempCanvas || entries.length === 0) return;

    const ctx = tempCanvas.getContext('2d');
    if (!ctx) return;

    const timestamps = parseTimestamps();
    const tempData = entries.map((e) => e.cpu_temp);

    tempChart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets: [
          {
            label: 'CPU Temperature (°C)',
            data: tempData,
            borderColor: 'rgb(239, 68, 68)',
            backgroundColor: 'rgba(239, 68, 68, 0.1)',
            tension: 0.4,
            pointRadius: 0.3,
            pointHoverRadius: 4,
            fill: true,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
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
            ticks: { font: { size: 10 } },
            grid: { color: 'rgba(0, 0, 0, 0.05)' },
          },
        },
      },
    });
  }

  function updateCharts() {
    if (entries.length === 0) return;

    const timestamps = parseTimestamps();

    if (cpuChart) {
      cpuChart.data.labels = timestamps;
      cpuChart.data.datasets[0].data = entries.map((e) => e.cpu_usage);
      cpuChart.update('none');
    }

    if (ramChart) {
      ramChart.data.labels = timestamps;
      ramChart.data.datasets[0].data = entries.map((e) => e.ram_usage);
      ramChart.update('none');
    }

    if (tempChart) {
      tempChart.data.labels = timestamps;
      tempChart.data.datasets[0].data = entries.map((e) => e.cpu_temp);
      tempChart.update('none');
    }
  }

  onMount(() => {
    createCpuChart();
    createRamChart();
    createTempChart();
  });

  onDestroy(() => {
    if (cpuChart) cpuChart.destroy();
    if (ramChart) ramChart.destroy();
    if (tempChart) tempChart.destroy();
  });

  $: if (entries) {
    updateCharts();
  }

  $: latestEntry = entries.length > 0 ? entries[entries.length - 1] : null;
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

  <div class="chart-grid">
    <div class="chart-card">
      <h3>CPU Usage (%)</h3>
      <div class="graph-container">
        <canvas bind:this={cpuCanvas}></canvas>
      </div>
    </div>

    <div class="chart-card">
      <h3>RAM Usage (%)</h3>
      <div class="graph-container">
        <canvas bind:this={ramCanvas}></canvas>
      </div>
    </div>

    <div class="chart-card">
      <h3>CPU Temperature (°C)</h3>
      <div class="graph-container">
        <canvas bind:this={tempCanvas}></canvas>
      </div>
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

  .chart-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1.5rem;
  }

  .chart-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }

  .chart-card h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
  }

  .graph-container {
    width: 100%;
    height: 200px;
    position: relative;
  }

  canvas {
    max-width: 100%;
    max-height: 100%;
  }
</style>
