<script lang="ts">
  import { onMount, onDestroy } from "svelte";
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
    BarController,
    BarElement,
  } from "chart.js";
  import "chartjs-adapter-date-fns";
  import type { SensorReading } from "../api";
  import { formatDateTime, timeToken, hourCycle } from "../utils/time";

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
    BarController,
    BarElement
  );

  let {
    readings = [],
    selectedMetric = null
  }: {
    readings?: SensorReading[];
    selectedMetric?: "temperature" | "humidity" | "presence" | null;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let chart = $state<Chart | null>(null);

  function getDatasets() {
    const timestamps = readings.map((r) => r.timestamp.getTime());

    // Check if this is a presence sensor
    const isPresenceSensor = readings.length > 0 && readings[0].type === 'presence';

    if (isPresenceSensor) {
      const presenceData = readings.map((r) => r.type === 'presence' ? (r.occupied ? 1 : 0) : 0);

      const allDatasets = [
        {
          type: 'bar' as const,
          label: 'Presence',
          data: presenceData,
          borderColor: "rgb(251, 191, 36)",
          backgroundColor: "rgba(251, 191, 36, 0.7)",
          yAxisID: "y",
          barThickness: 'flex' as const,
          maxBarThickness: 40,
          categoryPercentage: 1.0,
          barPercentage: 1.0,
          hidden: selectedMetric !== null && selectedMetric !== "presence",
        },
      ];

      return { timestamps, datasets: allDatasets };
    }

    // Temperature/humidity sensor
    const temperatures = readings.map((r) => r.type === 'temp_humidity' ? r.temperature : 0);
    const humidities = readings.map((r) => r.type === 'temp_humidity' ? r.humidity : 0);

    const POINT_RADUIS = 0.3;
    const POINT_RADIUS_HOVER = 1;
    const allDatasets = [
      {
        label: "Temperature (°C)",
        data: temperatures,
        borderColor: "rgb(59, 130, 246)",
        backgroundColor: "rgba(59, 130, 246, 0.1)",
        yAxisID: "y",
        tension: 0.4,
        pointRadius: POINT_RADUIS,
        pointHoverRadius: POINT_RADIUS_HOVER,
        hidden: selectedMetric !== null && selectedMetric !== "temperature",
      },
      {
        label: "Humidity (%)",
        data: humidities,
        borderColor: "rgb(34, 197, 94)",
        backgroundColor: "rgba(34, 197, 94, 0.1)",
        yAxisID: selectedMetric === "humidity" ? "y" : "y1",
        tension: 0.4,
        pointRadius: POINT_RADUIS,
        pointHoverRadius: POINT_RADIUS_HOVER,
        hidden: selectedMetric !== null && selectedMetric !== "humidity",
      },
    ];

    return { timestamps, datasets: allDatasets };
  }

  function roundAwayFromZeroTo5(n: number) {
    if (n === 0) return 0;
    const sign = Math.sign(n);
    const absN = Math.abs(n);
    const roundedAbs = Math.ceil(absN / 5) * 5;
    return sign * roundedAbs;
  }

  function computeGraphTemperatureMinMax(
    datasets: { label: string; data: number[] }[]
  ) {
    const temperatureDataSet = datasets.find(
      ({ label }) => label === "Temperature (°C)"
    );

    if (!temperatureDataSet) return { min: 0, max: 40 };

    const minTemperature = Math.min(...temperatureDataSet.data);
    const maxTemperature = Math.max(...temperatureDataSet.data);
    return {
      min: Math.min(roundAwayFromZeroTo5(minTemperature), 0),
      max: Math.max(roundAwayFromZeroTo5(maxTemperature), 40),
    };
  }

  function createChart() {
    if (!canvas || readings.length === 0) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const { timestamps, datasets } = getDatasets();
    const isPresenceSensor = readings.length > 0 && readings[0].type === 'presence';

    chart = new Chart(ctx, {
      type: "line",
      data: {
        labels: timestamps,
        datasets,
      },
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
                  return formatDateTime(date);
                }
                return "";
              },
              label: (context) => {
                if (isPresenceSensor) {
                  return context.parsed.y === 1 ? "Occupied" : "Clear";
                }
                return context.dataset.label + ": " + (context.parsed.y ?? 0).toFixed(1);
              },
            },
          },
        },
        scales: {
          x: {
            type: "time",
            time: {
              tooltipFormat: "MMM d, HH:mm",
              displayFormats: {
                hour: timeToken(),
                day: "MMM d",
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
            type: "linear",
            display: true,
            position: "left",
            title: {
              display: true,
              text: isPresenceSensor ? "Presence" : (selectedMetric === "humidity" ? "%" : "°C"),
              font: {
                size: 10,
              },
            },
            grid: {
              // Emphasize the 0°C freezing line on the temperature axis (#2).
              color: (ctx: any) =>
                ctx.tick?.value === 0 && !isPresenceSensor && selectedMetric !== "humidity"
                  ? "rgba(107, 114, 128, 0.9)"
                  : "rgba(0, 0, 0, 0.05)",
              lineWidth: (ctx: any) =>
                ctx.tick?.value === 0 && !isPresenceSensor && selectedMetric !== "humidity"
                  ? 2
                  : 1,
            },
            ticks: {
              font: {
                size: 10,
              },
              callback: function(value) {
                if (isPresenceSensor) {
                  return value === 1 ? "Occupied" : (value === 0 ? "Clear" : "");
                }
                return value;
              },
            },
            ...(isPresenceSensor ? { min: 0, max: 1, ticks: { stepSize: 1 } } : computeGraphTemperatureMinMax(datasets)),
          },
          y1: {
            type: "linear",
            display: !isPresenceSensor && selectedMetric === null,
            position: "right",
            title: {
              display: true,
              text: "%",
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
      yAxis.title.text = selectedMetric === "humidity" ? "%" : "°C";
    }

    const { max: maxTemperatureLimit, min: minTemperatureLimit } =
      computeGraphTemperatureMinMax(datasets);
    if (yAxis) {
      yAxis.min = selectedMetric === null ? minTemperatureLimit : undefined;
      yAxis.max = selectedMetric === null ? maxTemperatureLimit : undefined;
    }

    // Show/hide secondary Y-axis and set fixed scale
    const y1Axis = chart.options.scales?.y1 as any;
    if (y1Axis) {
      y1Axis.display = selectedMetric === null;
    }

    chart.update("none"); // Update without animation for performance
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

  $effect(() => {
    if (chart && (readings || selectedMetric !== undefined)) {
      updateChart();
    }
  });

  // Refresh axis labels when the clock-format preference changes (#16).
  $effect(() => {
    void $hourCycle; // reactive dependency
    if (chart) {
      const xAxis = chart.options.scales?.x as any;
      if (xAxis?.time) {
        xAxis.time.displayFormats = { hour: timeToken(), day: "MMM d" };
        chart.update("none");
      }
    }
  });
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
