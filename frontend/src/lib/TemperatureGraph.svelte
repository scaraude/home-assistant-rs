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
    Filler,
  } from 'chart.js';
  import 'chartjs-adapter-date-fns';
  import type { SensorReading } from './api';

  // Register Chart.js components
  Chart.register(
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    TimeScale,
    Title,
    Tooltip,
    Legend,
    Filler
  );

  export let readings: SensorReading[] = [];

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  function createChart() {
    if (!canvas || readings.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Prepare data
    const timestamps = readings.map((r) => r.timestamp * 1000); // Convert to milliseconds
    const temperatures = readings.map((r) => r.temperature);
    const humidities = readings.map((r) => r.humidity);

    chart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets: [
          {
            label: 'Temperature (°C)',
            data: temperatures,
            borderColor: 'rgb(59, 130, 246)',
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
            yAxisID: 'y',
            tension: 0.4,
            pointRadius: 2,
            pointHoverRadius: 4,
          },
          {
            label: 'Humidity (%)',
            data: humidities,
            borderColor: 'rgb(34, 197, 94)',
            backgroundColor: 'rgba(34, 197, 94, 0.1)',
            yAxisID: 'y1',
            tension: 0.4,
            pointRadius: 2,
            pointHoverRadius: 4,
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
              font: {
                size: 11,
              },
            },
          },
          tooltip: {
            enabled: true,
            callbacks: {
              title: (items) => {
                if (items.length > 0) {
                  const date = new Date(items[0].parsed.x);
                  return date.toLocaleString();
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
              displayFormats: {
                hour: 'HH:mm',
                day: 'MMM d',
              },
            },
            grid: {
              display: false,
            },
            ticks: {
              maxRotation: 0,
              font: {
                size: 10,
              },
            },
          },
          y: {
            type: 'linear',
            display: true,
            position: 'left',
            title: {
              display: true,
              text: '°C',
              font: {
                size: 10,
              },
            },
            grid: {
              color: 'rgba(0, 0, 0, 0.05)',
            },
            ticks: {
              font: {
                size: 10,
              },
            },
          },
          y1: {
            type: 'linear',
            display: true,
            position: 'right',
            title: {
              display: true,
              text: '%',
              font: {
                size: 10,
              },
            },
            grid: {
              drawOnChartArea: false,
            },
            ticks: {
              font: {
                size: 10,
              },
            },
          },
        },
      },
    });
  }

  function updateChart() {
    if (!chart || readings.length === 0) return;

    const timestamps = readings.map((r) => r.timestamp * 1000);
    const temperatures = readings.map((r) => r.temperature);
    const humidities = readings.map((r) => r.humidity);

    chart.data.labels = timestamps;
    chart.data.datasets[0].data = temperatures;
    chart.data.datasets[1].data = humidities;
    chart.update('none'); // Update without animation for performance
  }

  onMount(() => {
    createChart();
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  $: if (chart && readings.length > 0) {
    updateChart();
  }
</script>

<div class="graph-container">
  <canvas bind:this={canvas}></canvas>
</div>

<style>
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
