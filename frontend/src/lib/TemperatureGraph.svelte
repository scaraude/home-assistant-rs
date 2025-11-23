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
  export let selectedMetric: 'temperature' | 'humidity' | 'battery' | null = null;

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  function getDatasets() {
    const timestamps = readings.map((r) => r.timestamp * 1000);
    const temperatures = readings.map((r) => r.temperature);
    const humidities = readings.map((r) => r.humidity);
    const batteries = readings.map((r) => r.battery ?? 0);

    const POINT_RADUIS = 0.3;
    const POINT_RADIUS_HOVER = 1;
    const allDatasets = [
      {
        label: 'Temperature (°C)',
        data: temperatures,
        borderColor: 'rgb(59, 130, 246)',
        backgroundColor: 'rgba(59, 130, 246, 0.1)',
        yAxisID: 'y',
        tension: 0.4,
        pointRadius: POINT_RADUIS,
        pointHoverRadius: POINT_RADIUS_HOVER,
        hidden: selectedMetric !== null && selectedMetric !== 'temperature',
      },
      {
        label: 'Humidity (%)',
        data: humidities,
        borderColor: 'rgb(34, 197, 94)',
        backgroundColor: 'rgba(34, 197, 94, 0.1)',
        yAxisID: selectedMetric === 'humidity' ? 'y' : 'y1',
        tension: 0.4,
        pointRadius: POINT_RADUIS,
        pointHoverRadius: POINT_RADIUS_HOVER,
        hidden: selectedMetric !== null && selectedMetric !== 'humidity',
      },
      {
        label: 'Battery (%)',
        data: batteries,
        borderColor: 'rgb(234, 179, 8)',
        backgroundColor: 'rgba(234, 179, 8, 0.1)',
        yAxisID: selectedMetric === 'battery' ? 'y' : 'y1',
        tension: 0.4,
        pointRadius: POINT_RADUIS,
        pointHoverRadius: POINT_RADIUS_HOVER,
        hidden: selectedMetric !== null && selectedMetric !== 'battery',
      },
    ];

    return { timestamps, datasets: allDatasets };
  }

  function createChart() {
    if (!canvas || readings.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { timestamps, datasets } = getDatasets();

    chart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: timestamps,
        datasets,
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
                if (items.length > 0 && items[0].parsed.x !== null) {
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
              text: selectedMetric === 'humidity' ? '%' : selectedMetric === 'battery' ? '%' : '°C',
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
            min: selectedMetric === null ? 0 : undefined,
            max: selectedMetric === null ? 40 : undefined,
          },
          y1: {
            type: 'linear',
            display: selectedMetric === null,
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
            min: 0,
            max: 100,
          },
        },
      },
    });
  }

  function updateChart() {
    if (!chart || readings.length === 0) return;

    const { timestamps, datasets } = getDatasets();

    chart.data.labels = timestamps;
    chart.data.datasets = datasets;

    // Update Y-axis label and scale based on selected metric
    const yAxis = chart.options.scales?.y as any;
    if (yAxis?.title) {
      yAxis.title.text =
        selectedMetric === 'humidity' ? '%' :
        selectedMetric === 'battery' ? '%' : '°C';
    }
    if (yAxis) {
      yAxis.min = selectedMetric === null ? 0 : undefined;
      yAxis.max = selectedMetric === null ? 40 : undefined;
    }

    // Show/hide secondary Y-axis and set fixed scale
    const y1Axis = chart.options.scales?.y1 as any;
    if (y1Axis) {
      y1Axis.display = selectedMetric === null;
    }

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

  $: if (chart && (readings || selectedMetric !== undefined)) {
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
