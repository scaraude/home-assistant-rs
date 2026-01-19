<script lang="ts">
  import { onDestroy } from "svelte";
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
  } from "chart.js";
  import zoomPlugin from "chartjs-plugin-zoom";
  import "chartjs-adapter-date-fns";
  import type { SensorUIConfig } from "./stores/graphConfig";
  import { dataCache } from "./stores/dataCache";

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
    Filler,
    zoomPlugin
  );

  let {
    sensors = [],
    metric = 'temperature',
    timeRange = '24h',
    metricType = 'temperature'
  }: {
    sensors?: SensorUIConfig[];
    metric?: 'temperature' | 'humidity' | 'both';
    timeRange?: string;
    metricType?: 'temperature' | 'power';
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let chart = $state<Chart | null>(null);

  // Derived: visible sensors
  let visibleSensors = $derived(sensors.filter(s => s.visible));

  // Derived: get device info map for names
  let deviceMap = $derived(
    new Map($dataCache.sensors.devices.map(d => [d.device_id, d]))
  );

  // Derived: readings filtered by visible sensors
  let visibleReadings = $derived(
    $dataCache.sensors.readings.filter(r =>
      visibleSensors.some(s => s.deviceId === r.device_id)
    )
  );

  // Build datasets for visible sensors
  function buildDatasets() {
    const POINT_RADIUS = 0.3;
    const POINT_RADIUS_HOVER = 1;

    const datasets = [];

    for (const sensor of visibleSensors) {
      const sensorReadings = visibleReadings
        .filter(r => r.device_id === sensor.deviceId)
        .sort((a, b) => a.timestamp - b.timestamp);

      if (sensorReadings.length === 0) continue;

      const deviceName = deviceMap.get(sensor.deviceId)?.name || sensor.deviceId;

      // Power metric (for energy meters)
      if (metricType === 'power') {
        const timestamps = sensorReadings.map(r => r.timestamp * 1000);
        const powerValues = sensorReadings.map(r => r.type === 'energy_meter' ? r.power : 0);

        datasets.push({
          label: deviceName,
          data: timestamps.map((t, i) => ({ x: t, y: powerValues[i] })),
          borderColor: sensor.color,
          backgroundColor: `${sensor.color}20`,
          yAxisID: 'y',
          tension: 0.4,
          pointRadius: POINT_RADIUS,
          pointHoverRadius: POINT_RADIUS_HOVER,
          fill: false,
        });
      }
      // Temperature/Humidity metrics
      else {
        const timestamps = sensorReadings.map(r => r.timestamp * 1000);
        const temperatures = sensorReadings.map(r => r.type === 'temp_humidity' ? r.temperature : 0);
        const humidities = sensorReadings.map(r => r.type === 'temp_humidity' ? r.humidity : 0);

        // Temperature dataset
        if (metric === 'temperature' || metric === 'both') {
          datasets.push({
            label: `${deviceName} (Temp)`,
            data: timestamps.map((t, i) => ({ x: t, y: temperatures[i] })),
            borderColor: sensor.color,
            backgroundColor: `${sensor.color}20`, // 20 = 12.5% opacity in hex
            yAxisID: 'y',
            tension: 0.4,
            pointRadius: POINT_RADIUS,
            pointHoverRadius: POINT_RADIUS_HOVER,
            fill: false,
          });
        }

        // Humidity dataset
        if (metric === 'humidity' || metric === 'both') {
          datasets.push({
            label: `${deviceName} (Humidity)`,
            data: timestamps.map((t, i) => ({ x: t, y: humidities[i] })),
            borderColor: sensor.color,
            backgroundColor: `${sensor.color}20`,
            yAxisID: metric === 'both' ? 'y1' : 'y',
            tension: 0.4,
            pointRadius: POINT_RADIUS,
            pointHoverRadius: POINT_RADIUS_HOVER,
            fill: false,
            borderDash: metric === 'both' ? [5, 5] : [], // Dashed line for humidity when both shown
          });
        }
      }
    }

    return datasets;
  }

  function getTimeDisplayFormats(range: string) {
    switch (range) {
      case '24h':
        return { hour: 'HH:mm', day: 'HH:mm' };
      case '1w':
        return { day: 'MMM dd', hour: 'MMM dd' };
      case '1m':
        return { day: 'MMM dd', week: 'MMM dd' };
      case '1y':
        return { month: 'MMM yyyy', week: 'MMM yyyy' };
      default:
        return { hour: 'HH:mm', day: 'MMM dd' };
    }
  }

  function createChart() {
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const datasets = buildDatasets();

    chart = new Chart(ctx, {
      type: "line",
      data: { datasets },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
          mode: "index",
          intersect: false,
        },
        plugins: {
          legend: {
            display: true,
            position: "top",
            labels: {
              usePointStyle: true,
              padding: 15,
              font: { size: 12 },
              boxWidth: 8,
              boxHeight: 8,
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
                return "";
              },
            },
          },
          zoom: {
            pan: {
              enabled: true,
              mode: 'x',
            },
            zoom: {
              wheel: {
                enabled: true,
                speed: 0.1,
              },
              pinch: {
                enabled: true,
              },
              mode: 'x',
            },
            limits: {
              x: { min: 'original', max: 'original' },
            },
          },
        },
        scales: {
          x: {
            type: "time",
            time: {
              tooltipFormat: "PPpp",
              displayFormats: getTimeDisplayFormats(timeRange),
            },
            grid: {
              display: false,
            },
            ticks: {
              maxRotation: 0,
              font: { size: 11 },
            },
          },
          y: {
            type: "linear",
            display: true,
            position: "left",
            title: {
              display: true,
              text: metricType === 'power' ? 'W' : (metric === 'humidity' ? '%' : '°C'),
              font: { size: 12 },
            },
            grid: {
              color: "rgba(0, 0, 0, 0.05)",
            },
            ticks: {
              font: { size: 11 },
            },
          },
          y1: {
            type: "linear",
            display: metric === 'both',
            position: "right",
            title: {
              display: true,
              text: "%",
              font: { size: 12 },
            },
            grid: {
              drawOnChartArea: false,
            },
            ticks: {
              font: { size: 11 },
            },
            min: 0,
            max: 100,
          },
        },
      },
    });
  }

  function updateChart() {
    if (!chart) return;

    const datasets = buildDatasets();
    chart.data.datasets = datasets;

    // Update Y-axis configuration
    const yAxis = chart.options.scales?.y as any;
    if (yAxis?.title) {
      yAxis.title.text = metricType === 'power' ? 'W' : (metric === 'humidity' ? '%' : '°C');
    }

    const y1Axis = chart.options.scales?.y1 as any;
    if (y1Axis) {
      y1Axis.display = metric === 'both';
    }

    // Update time display formats
    const xAxis = chart.options.scales?.x as any;
    if (xAxis?.time) {
      xAxis.time.displayFormats = getTimeDisplayFormats(timeRange);
    }

    chart.update('none');
  }

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  // Create or update chart when dependencies change
  $effect(() => {
    // Destroy chart if no visible sensors or readings
    if (visibleSensors.length === 0 || visibleReadings.length === 0) {
      if (chart) {
        chart.destroy();
        chart = null;
      }
      return;
    }

    // Need canvas to create chart
    if (!canvas) {
      return;
    }

    // Create or update chart
    if (!chart) {
      createChart();
    } else {
      updateChart();
    }
  });
</script>

<div class="chart-container">
  {#if visibleSensors.length === 0}
    <div class="no-data">
      <p>No sensors selected. Click on a sensor to view its data.</p>
    </div>
  {:else if visibleReadings.length === 0}
    <div class="no-data">
      <p>No data available for the selected time range.</p>
    </div>
  {:else}
    <canvas bind:this={canvas}></canvas>
  {/if}
</div>

<style>
  .chart-container {
    width: 100%;
    height: 100%;
    position: relative;
    background: white;
    border-radius: 8px;
    padding: 1rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  canvas {
    max-width: 100%;
    max-height: 100%;
  }

  .no-data {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #6b7280;
    font-size: 0.9375rem;
  }
</style>
