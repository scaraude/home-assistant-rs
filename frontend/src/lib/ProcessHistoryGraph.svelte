<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
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
  import { fetchProcessHistory, type ProcessMonitorEntry } from './api';

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

  export let processName: string;
  export let pid: string;
  export let timeRange: '1h' | '6h' | '24h' | 'all' = '24h';

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;
  let allEntries: ProcessMonitorEntry[] = [];
  let filteredEntries: ProcessMonitorEntry[] = [];
  let loading = true;
  let error: string | null = null;

  // Map time ranges to hours
  const timeRangeToHours: Record<typeof timeRange, number> = {
    '1h': 1,
    '6h': 6,
    '24h': 24,
    'all': Infinity,
  };

  // Filter entries based on time range
  function filterEntriesByTimeRange(entries: ProcessMonitorEntry[]): ProcessMonitorEntry[] {
    if (timeRange === 'all') return entries;

    const hoursToShow = timeRangeToHours[timeRange];
    const cutoffTime = Date.now() - (hoursToShow * 60 * 60 * 1000);

    return entries.filter(e => new Date(e.timestamp).getTime() >= cutoffTime);
  }

  // Reactive statement to filter entries when timeRange changes
  $: filteredEntries = filterEntriesByTimeRange(allEntries);

  // Load process history from backend on mount
  async function loadProcessHistory() {
    loading = true;
    error = null;
    try {
      const entries = await fetchProcessHistory(processName, pid, 10000);
      allEntries = entries.sort(
        (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
      );
      loading = false;

      // Wait for DOM update, then create chart
      await tick();
      if (filteredEntries.length > 0) {
        createChart();
      }
    } catch (e) {
      console.error('Failed to load process history:', e);
      error = 'Failed to load process history';
      loading = false;
    }
  }

  function updateChartData() {
    if (!chart || filteredEntries.length === 0) return;

    const timestamps = filteredEntries.map((e) => new Date(e.timestamp).getTime());
    const cpuData = filteredEntries.map((e) => e.cpu);
    const ramData = filteredEntries.map((e) => e.ram);

    chart.data.labels = timestamps;
    chart.data.datasets[0].data = cpuData;
    chart.data.datasets[1].data = ramData;
    chart.update('none'); // Update without animation for instant response
  }

  // Reactive statement to update chart when filtered data changes
  $: if (chart && filteredEntries.length > 0) {
    updateChartData();
  }

  function createChart() {
    if (!canvas || filteredEntries.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const timestamps = filteredEntries.map((e) => new Date(e.timestamp).getTime());
    const cpuData = filteredEntries.map((e) => e.cpu);
    const ramData = filteredEntries.map((e) => e.ram);

    chart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets: [
          {
            label: 'CPU (%)',
            data: cpuData,
            borderColor: 'rgb(59, 130, 246)',
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
            yAxisID: 'y',
            tension: 0.4,
            pointRadius: 2,
            pointHoverRadius: 4,
            fill: false,
          },
          {
            label: 'RAM (MB)',
            data: ramData,
            borderColor: 'rgb(34, 197, 94)',
            backgroundColor: 'rgba(34, 197, 94, 0.1)',
            yAxisID: 'y1',
            tension: 0.4,
            pointRadius: 2,
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
              padding: 10,
              font: { size: 11 },
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
        },
        scales: {
          x: {
            type: 'time',
            time: {
              tooltipFormat: 'MMM d, HH:mm',
              displayFormats: { hour: 'HH:mm', day: 'MMM d' },
            },
            grid: { display: false },
            ticks: { maxRotation: 0, font: { size: 9 } },
          },
          y: {
            type: 'linear',
            display: true,
            position: 'left',
            min: 0,
            title: {
              display: true,
              text: 'CPU (%)',
              font: { size: 10 },
            },
            ticks: { font: { size: 9 } },
            grid: { color: 'rgba(0, 0, 0, 0.05)' },
          },
          y1: {
            type: 'linear',
            display: true,
            position: 'right',
            min: 0,
            title: {
              display: true,
              text: 'RAM (MB)',
              font: { size: 10 },
            },
            ticks: { font: { size: 9 } },
            grid: { drawOnChartArea: false },
          },
        },
      },
    });
  }

  onMount(() => {
    loadProcessHistory();
  });

  onDestroy(() => {
    if (chart) chart.destroy();
  });
</script>

<div class="process-history">
  {#if loading}
    <div class="loading">Loading process history...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if filteredEntries.length === 0}
    <div class="no-data">No historical data available for this process</div>
  {:else}
    <div class="graph-container">
      <canvas bind:this={canvas}></canvas>
    </div>
  {/if}
</div>

<style>
  .process-history {
    padding: 1rem;
    background: #f9fafb;
    border-top: 1px solid #e5e7eb;
  }

  .loading,
  .error,
  .no-data {
    padding: 1rem;
    text-align: center;
    color: #6b7280;
    font-size: 0.875rem;
  }

  .error {
    color: #dc2626;
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
