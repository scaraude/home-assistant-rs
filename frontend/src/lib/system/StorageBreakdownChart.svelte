<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Chart, DoughnutController, ArcElement, Tooltip, Legend } from "chart.js";
  import type { StorageBreakdown } from "../api";

  Chart.register(DoughnutController, ArcElement, Tooltip, Legend);

  let { breakdown = null }: { breakdown?: StorageBreakdown | null } = $props();

  let chartCanvas = $state<HTMLCanvasElement>();
  let chart = $state<Chart | null>(null);

  const palette = [
    "#3b82f6",
    "#22c55e",
    "#f97316",
    "#a855f7",
    "#0ea5e9",
    "#ef4444",
    "#64748b",
    "#94a3b8",
  ];

  function formatBytes(bytes: number): string {
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let index = 0;
    while (value >= 1024 && index < units.length - 1) {
      value /= 1024;
      index += 1;
    }
    return `${value.toFixed(value < 10 ? 2 : 1)} ${units[index]}`;
  }

  function buildDataset() {
    if (!breakdown || breakdown.categories.length === 0) {
      return null;
    }
    const labels = breakdown.categories.map((category) => category.label);
    const data = breakdown.categories.map((category) => category.bytes);
    const colors = data.map((_, index) => palette[index % palette.length]);
    return { labels, data, colors };
  }

  function createChart() {
    if (!chartCanvas) return;
    const dataset = buildDataset();
    if (!dataset) return;

    const ctx = chartCanvas.getContext("2d");
    if (!ctx) return;

    chart = new Chart(ctx, {
      type: "doughnut",
      data: {
        labels: dataset.labels,
        datasets: [
          {
            data: dataset.data,
            backgroundColor: dataset.colors,
            borderWidth: 0,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        cutout: "60%",
        plugins: {
          legend: {
            position: "right",
            labels: {
              boxWidth: 14,
              boxHeight: 14,
              usePointStyle: true,
              padding: 12,
              font: { size: 11 },
              generateLabels: (chartInstance) => {
                const data = chartInstance.data;
                const dataset = data.datasets[0];
                const labels = data.labels || [];
                return labels.map((label, index) => {
                  const value = Array.isArray(dataset.data)
                    ? (dataset.data[index] as number)
                    : 0;
                  const color = Array.isArray(dataset.backgroundColor)
                    ? dataset.backgroundColor[index]
                    : "#94a3b8";
                  return {
                    text: `${label}: ${formatBytes(value)}`,
                    fillStyle: color,
                    strokeStyle: color,
                    lineWidth: 0,
                    hidden: !chartInstance.getDataVisibility(index),
                    index,
                  };
                });
              },
            },
          },
          tooltip: {
            callbacks: {
              label: (item) => {
                const value = item.parsed ?? 0;
                const total = breakdown?.total_bytes ?? 1;
                const percent = ((value / total) * 100).toFixed(1);
                return `${item.label}: ${formatBytes(value)} (${percent}%)`;
              },
            },
          },
        },
      },
    });
  }

  function updateChart() {
    if (!chart) return;
    const dataset = buildDataset();
    if (!dataset) return;

    chart.data.labels = dataset.labels;
    chart.data.datasets[0].data = dataset.data;
    chart.data.datasets[0].backgroundColor = dataset.colors;
    chart.update("none");
  }

  onMount(() => {
    if (breakdown) {
      createChart();
    }
  });

  onDestroy(() => {
    if (chart) chart.destroy();
  });

  $effect(() => {
    if (!breakdown) return;
    if (!chart) {
      createChart();
    } else {
      updateChart();
    }
  });
</script>

<div class="storage-card">
  <div class="storage-header">
    <div>
      <h3>Storage Breakdown</h3>
      {#if breakdown}
        <p class="storage-subtitle">
          {formatBytes(breakdown.used_bytes)} used of {formatBytes(breakdown.total_bytes)} on
          {breakdown.mount}
        </p>
      {/if}
    </div>
    {#if breakdown?.ram_storage}
      <div class="ram-summary">
        <div class="ram-label">RAM storage</div>
        <div class="ram-value">
          {formatBytes(breakdown.ram_storage.used_bytes)} /
          {formatBytes(breakdown.ram_storage.total_bytes)}
        </div>
      </div>
    {/if}
  </div>

  {#if breakdown}
    <div class="chart-wrapper">
      <canvas bind:this={chartCanvas}></canvas>
    </div>
  {:else}
    <p class="storage-empty">Storage data not available.</p>
  {/if}
</div>

<style>
  .storage-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }

  .storage-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .storage-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
  }

  .storage-subtitle {
    margin: 0.35rem 0 0;
    font-size: 0.75rem;
    color: #6b7280;
  }

  .ram-summary {
    text-align: right;
    font-size: 0.75rem;
    color: #6b7280;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 0.4rem 0.6rem;
  }

  .ram-label {
    font-weight: 600;
    color: #475569;
  }

  .ram-value {
    margin-top: 0.25rem;
  }

  .chart-wrapper {
    position: relative;
    height: 320px;
  }

  .storage-empty {
    font-size: 0.875rem;
    color: #9ca3af;
  }
</style>
