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
  import type { SensorUIConfig } from "../stores/graphConfig";
  import { TIME_RANGE_HOURS } from "../stores/graphConfig";
  import { fetchAggregatedReadings } from "../api";
  import { dataCache } from "../stores/dataCache";

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
    zoomPlugin,
  );

  let {
    sensors = [],
    metric = "temperature",
    timeRange = "24h",
    metricType = "temperature",
  }: {
    sensors?: SensorUIConfig[];
    metric?: "temperature" | "humidity" | "both";
    timeRange?: string;
    metricType?: "temperature" | "power";
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let chart = $state<Chart | null>(null);
  let zoomRange = $state<{ startSec: number; endSec: number } | null>(null);

  const TARGET_POINTS = 1500;
  const MIN_BUCKET_SECONDS = 1;
  const inflightRequests = new Set<string>();

  // Derived: visible sensors
  let visibleSensors = $derived(sensors.filter((s) => s.visible));

  // Derived: get device info map for names
  let deviceMap = $derived(
    new Map($dataCache.sensors.devices.map((d) => [d.device_id, d])),
  );

  // Derived: aggregated series cache
  let graphSeries = $derived($dataCache.sensors.graphSeries);

  function getDefaultRangeSeconds(range: string) {
    const hours = TIME_RANGE_HOURS[range as keyof typeof TIME_RANGE_HOURS] ?? 24;
    const endSec = Math.floor(Date.now() / 1000);
    const startSec = endSec - hours * 3600;
    return { startSec, endSec };
  }

  function computeBucketSeconds(rangeSeconds: number) {
    return Math.max(MIN_BUCKET_SECONDS, Math.ceil(rangeSeconds / TARGET_POINTS));
  }

  const activeRange = $derived.by(() => {
    if (zoomRange) {
      return zoomRange;
    }
    return getDefaultRangeSeconds(timeRange);
  });

  const activeBucketSeconds = $derived.by(() =>
    computeBucketSeconds(activeRange.endSec - activeRange.startSec),
  );

  function hasCoverage(
    deviceId: string,
    bucketSeconds: number,
    startSec: number,
    endSec: number,
  ): boolean {
    const windows = graphSeries[deviceId] || [];
    return windows.some(
      (window) =>
        window.bucketSeconds === bucketSeconds &&
        window.start <= startSec &&
        window.end >= endSec,
    );
  }

  function getCachedReadings(
    deviceId: string,
    bucketSeconds: number,
    startSec: number,
    endSec: number,
  ) {
    const windows = graphSeries[deviceId] || [];
    const matching = windows.filter(
      (window) =>
        window.bucketSeconds === bucketSeconds &&
        window.start <= endSec &&
        window.end >= startSec,
    );

    if (matching.length === 0) {
      return [];
    }

    const readings = matching.flatMap((window) => window.readings);
    const startMs = startSec * 1000;
    const endMs = endSec * 1000;
    return readings
      .filter((reading) => {
        const ts = reading.timestamp.getTime();
        return ts >= startMs && ts <= endMs;
      })
      .sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
  }

  // Derived: readings filtered by visible sensors and active range
  let visibleReadings = $derived.by(() => {
    if (visibleSensors.length === 0) return [];
    const readings: typeof $dataCache.sensors.readings = [];
    for (const sensor of visibleSensors) {
      readings.push(
        ...getCachedReadings(
          sensor.deviceId,
          activeBucketSeconds,
          activeRange.startSec,
          activeRange.endSec,
        ),
      );
    }
    return readings;
  });

  const missingSeries = $derived.by(() => {
    if (visibleSensors.length === 0) return false;
    return visibleSensors.some((sensor) =>
      !hasCoverage(
        sensor.deviceId,
        activeBucketSeconds,
        activeRange.startSec,
        activeRange.endSec,
      )
    );
  });

  // Build datasets for visible sensors
  function buildDatasets() {
    const POINT_RADIUS = 0.3;
    const POINT_RADIUS_HOVER = 1;

    const datasets = [];

    for (const sensor of visibleSensors) {
      const sensorReadings = visibleReadings
        .filter((r) => r.device_id === sensor.deviceId)
        .sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());

      if (sensorReadings.length === 0) continue;

      const deviceName =
        deviceMap.get(sensor.deviceId)?.name || sensor.deviceId;

      // Power metric (for energy meters)
      if (metricType === "power") {
        const timestamps = sensorReadings.map((r) => r.timestamp.getTime());
        const powerValues = sensorReadings.map((r) =>
          r.type === "energy_meter" ? r.power : 0,
        );

        datasets.push({
          label: deviceName,
          data: timestamps.map((t, i) => ({ x: t, y: powerValues[i] })),
          borderColor: sensor.color,
          backgroundColor: `${sensor.color}20`,
          yAxisID: "y",
          tension: 0.4,
          pointRadius: POINT_RADIUS,
          pointHoverRadius: POINT_RADIUS_HOVER,
          fill: false,
        });
      }
      // Temperature/Humidity metrics
      else {
        const timestamps = sensorReadings.map((r) => r.timestamp.getTime());
        const temperatures = sensorReadings.map((r) =>
          r.type === "temp_humidity" ? r.temperature : 0,
        );
        const humidities = sensorReadings.map((r) =>
          r.type === "temp_humidity" ? r.humidity : 0,
        );

        // Temperature dataset
        if (metric === "temperature" || metric === "both") {
          datasets.push({
            label: `${deviceName} (Temp)`,
            data: timestamps.map((t, i) => ({ x: t, y: temperatures[i] })),
            borderColor: sensor.color,
            backgroundColor: `${sensor.color}20`, // 20 = 12.5% opacity in hex
            yAxisID: "y",
            tension: 0.4,
            pointRadius: POINT_RADIUS,
            pointHoverRadius: POINT_RADIUS_HOVER,
            fill: false,
          });
        }

        // Humidity dataset
        if (metric === "humidity" || metric === "both") {
          datasets.push({
            label: `${deviceName} (Humidity)`,
            data: timestamps.map((t, i) => ({ x: t, y: humidities[i] })),
            borderColor: sensor.color,
            backgroundColor: `${sensor.color}20`,
            yAxisID: metric === "both" ? "y1" : "y",
            tension: 0.4,
            pointRadius: POINT_RADIUS,
            pointHoverRadius: POINT_RADIUS_HOVER,
            fill: false,
            borderDash: metric === "both" ? [5, 5] : [], // Dashed line for humidity when both shown
          });
        }
      }
    }

    return datasets;
  }

  function updateZoomRangeFromChart(sourceChart: Chart) {
    const xScale = sourceChart.scales?.x as any;
    if (!xScale) return;
    const min = xScale.min;
    const max = xScale.max;
    if (typeof min !== "number" || typeof max !== "number") return;
    zoomRange = {
      startSec: Math.floor(min / 1000),
      endSec: Math.floor(max / 1000),
    };
  }

  async function requestSeries(
    deviceId: string,
    startSec: number,
    endSec: number,
    bucketSeconds: number,
  ) {
    const key = `${deviceId}:${bucketSeconds}:${startSec}:${endSec}`;
    if (inflightRequests.has(key)) {
      return;
    }
    inflightRequests.add(key);
    try {
      const result = await fetchAggregatedReadings(
        deviceId,
        new Date(startSec * 1000),
        new Date(endSec * 1000),
        bucketSeconds,
      );
      dataCache.setGraphSeries(deviceId, bucketSeconds, startSec, endSec, result.readings);
    } catch (error) {
      console.error("Failed to fetch aggregated readings:", error);
    } finally {
      inflightRequests.delete(key);
    }
  }

  $effect(() => {
    if (visibleSensors.length === 0) {
      return;
    }

    const startSec = activeRange.startSec;
    const endSec = activeRange.endSec;
    const bucketSeconds = activeBucketSeconds;

    for (const sensor of visibleSensors) {
      if (!hasCoverage(sensor.deviceId, bucketSeconds, startSec, endSec)) {
        void requestSeries(sensor.deviceId, startSec, endSec, bucketSeconds);
      }
    }
  });

  function getTimeDisplayFormats(range: string) {
    switch (range) {
      case "24h":
        return { hour: "HH:mm", day: "HH:mm" };
      case "1w":
        return { day: "MMM dd", hour: "MMM dd" };
      case "1m":
        return { day: "MMM dd", week: "MMM dd" };
      case "1y":
        return { month: "MMM yyyy", week: "MMM yyyy" };
      default:
        return { hour: "HH:mm", day: "MMM dd" };
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
          mode: "nearest",
          intersect: false,
        },
        plugins: {
          legend: {
            display: false,
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
              mode: "x",
              onPanComplete: ({ chart }) => {
                updateZoomRangeFromChart(chart);
              },
            },
            zoom: {
              wheel: {
                enabled: true,
                speed: 0.1,
              },
              pinch: {
                enabled: true,
              },
              mode: "x",
              onZoomComplete: ({ chart }) => {
                updateZoomRangeFromChart(chart);
              },
            },
            limits: {
              x: { min: "original", max: "original" },
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
              text:
                metricType === "power"
                  ? "W"
                  : metric === "humidity"
                    ? "%"
                    : "°C",
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
            display: metric === "both",
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
      yAxis.title.text =
        metricType === "power" ? "W" : metric === "humidity" ? "%" : "°C";
    }

    const y1Axis = chart.options.scales?.y1 as any;
    if (y1Axis) {
      y1Axis.display = metric === "both";
    }

    // Update time display formats
    const xAxis = chart.options.scales?.x as any;
    if (xAxis?.time) {
      xAxis.time.displayFormats = getTimeDisplayFormats(timeRange);
    }

    chart.update("none");
  }

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  let previousTimeRange: string | undefined = $state(undefined);
  $effect(() => {
    if (previousTimeRange !== undefined && timeRange !== previousTimeRange) {
      zoomRange = null;
    }
    previousTimeRange = timeRange;
  });

  // Create or update chart when dependencies change
  $effect(() => {
    // Destroy chart if no visible sensors
    if (visibleSensors.length === 0) {
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
  {:else}
    <canvas bind:this={canvas}></canvas>
    {#if missingSeries && visibleReadings.length === 0}
      <div class="no-data overlay">
        <p>Loading chart data...</p>
      </div>
    {:else if visibleReadings.length === 0}
      <div class="no-data overlay">
        <p>No data available for the selected time range.</p>
      </div>
    {/if}
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
    text-align: center;
  }

  .overlay {
    position: absolute;
    inset: 0;
    background: rgba(255, 255, 255, 0.9);
  }
</style>
