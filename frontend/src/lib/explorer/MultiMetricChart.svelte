<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    Chart,
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    TimeScale,
    Tooltip,
    Legend,
    Filler,
    type Plugin as ChartPlugin,
  } from "chart.js";
  import zoomPlugin from "chartjs-plugin-zoom";
  import "chartjs-adapter-date-fns";
  import { sensorsMemory } from "../memory";
  import { coversRange } from "../memory/rangeSet";
  import { TIME_RANGE_HOURS } from "../stores/graphConfig";
  import { chartTimeFormats, formatDateTime, hourCycle } from "../utils/time";
  import type { SensorReading } from "../api";
  import type { ExplorerSeries } from "./explorerStore";
  import { CATEGORY_BY_ID, metricValue } from "./metrics";

  Chart.register(
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    TimeScale,
    Tooltip,
    Legend,
    Filler,
    zoomPlugin,
  );

  let {
    series = [],
    timeRange = "24h",
  }: {
    series?: ExplorerSeries[];
    timeRange?: string;
  } = $props();

  const TARGET_POINTS = 1500;
  const MIN_BUCKET_SECONDS = 1;
  const inflightRequests = new Set<string>();

  let canvas = $state<HTMLCanvasElement>();
  let chart = $state<Chart | null>(null);
  let zoomRange = $state<{ startSec: number; endSec: number } | null>(null);

  // ---- presence bands overlay: full-height translucent bands drawn behind
  // the lines. This is the first "overlay" layer — event/note markers will
  // hook into the same plugin later (see chart.$overlays).
  interface Band {
    startMs: number;
    endMs: number;
    color: string;
  }
  const overlayPlugin: ChartPlugin = {
    id: "explorerOverlays",
    beforeDatasetsDraw(c) {
      const bands: Band[] = (c as any).$bands ?? [];
      if (bands.length === 0) return;
      const { ctx, chartArea, scales } = c;
      const x = scales.x;
      if (!x) return;
      ctx.save();
      for (const band of bands) {
        const left = x.getPixelForValue(band.startMs);
        const right = x.getPixelForValue(band.endMs);
        const clampedLeft = Math.max(chartArea.left, Math.min(chartArea.right, left));
        const clampedRight = Math.max(chartArea.left, Math.min(chartArea.right, right));
        const width = Math.max(1, clampedRight - clampedLeft);
        ctx.fillStyle = band.color;
        ctx.fillRect(clampedLeft, chartArea.top, width, chartArea.bottom - chartArea.top);
      }
      ctx.restore();
    },
  };
  Chart.register(overlayPlugin);

  let deviceMap = $derived(
    new Map($sensorsMemory.devices.map((d) => [d.device_id, d])),
  );
  let seriesByDevice = $derived($sensorsMemory.seriesByDevice);

  const lineSeries = $derived(series.filter((s) => CATEGORY_BY_ID[s.metric].kind === "line"));
  const presenceSeries = $derived(series.filter((s) => CATEGORY_BY_ID[s.metric].kind === "band"));

  // Distinct units among line series, in first-seen order → axis assignment.
  const units = $derived.by(() => {
    const seen: string[] = [];
    for (const s of lineSeries) {
      const unit = CATEGORY_BY_ID[s.metric].unit ?? "";
      if (unit && !seen.includes(unit)) seen.push(unit);
    }
    return seen;
  });
  function axisIdForUnit(unit: string): string {
    const idx = units.indexOf(unit);
    return idx <= 0 ? "y" : `y${idx}`;
  }

  function getDefaultRangeSeconds(range: string) {
    const hours = TIME_RANGE_HOURS[range as keyof typeof TIME_RANGE_HOURS] ?? 24;
    const endSec = Math.floor(Date.now() / 1000);
    return { startSec: endSec - hours * 3600, endSec };
  }
  function computeBucketSeconds(rangeSeconds: number) {
    return Math.max(MIN_BUCKET_SECONDS, Math.ceil(rangeSeconds / TARGET_POINTS));
  }

  const activeRange = $derived(zoomRange ?? getDefaultRangeSeconds(timeRange));
  const activeBucketSeconds = $derived(
    computeBucketSeconds(activeRange.endSec - activeRange.startSec),
  );

  function hasCoverage(deviceId: string, bucket: number, startSec: number, endSec: number) {
    const s = seriesByDevice[deviceId]?.[bucket];
    return s ? coversRange(s.ranges, startSec, endSec) : false;
  }
  function getCachedReadings(deviceId: string, bucket: number, startSec: number, endSec: number) {
    const s = seriesByDevice[deviceId]?.[bucket];
    if (!s) return [] as SensorReading[];
    const startMs = startSec * 1000;
    const endMs = endSec * 1000;
    return s.readings
      .filter((r) => {
        const ts = r.timestamp.getTime();
        return ts >= startMs && ts <= endMs;
      })
      .sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
  }

  // Fetch any missing series data for the active range.
  $effect(() => {
    if (series.length === 0) return;
    const { startSec, endSec } = activeRange;
    const bucket = activeBucketSeconds;
    for (const s of series) {
      if (!hasCoverage(s.deviceId, bucket, startSec, endSec)) {
        void requestSeries(s.deviceId, startSec, endSec, bucket);
      }
    }
  });

  async function requestSeries(deviceId: string, startSec: number, endSec: number, bucket: number) {
    const key = `${deviceId}:${bucket}:${startSec}:${endSec}`;
    if (inflightRequests.has(key)) return;
    inflightRequests.add(key);
    try {
      await sensorsMemory.ensureAggregatedSeries(deviceId, startSec, endSec, bucket);
    } catch (e) {
      console.error("Failed to fetch aggregated readings:", e);
    } finally {
      inflightRequests.delete(key);
    }
  }

  function buildDatasets() {
    const datasets: any[] = [];
    for (const s of lineSeries) {
      const cat = CATEGORY_BY_ID[s.metric];
      const readings = getCachedReadings(
        s.deviceId,
        activeBucketSeconds,
        activeRange.startSec,
        activeRange.endSec,
      );
      const points = readings
        .map((r) => ({ x: r.timestamp.getTime(), v: metricValue(r, s.metric) }))
        .filter((p) => p.v !== null)
        .map((p) => ({ x: p.x, y: p.v as number }));
      if (points.length === 0) continue;

      const name = deviceMap.get(s.deviceId)?.name ?? s.deviceId;
      datasets.push({
        label: `${name} · ${cat.label}`,
        data: points,
        borderColor: s.color,
        backgroundColor: `${s.color}20`,
        yAxisID: axisIdForUnit(cat.unit ?? ""),
        tension: 0.35,
        pointRadius: 0.3,
        pointHoverRadius: 3,
        fill: false,
        // dashed line for humidity so it reads apart from temperature of the
        // same device / color
        borderDash: s.metric === "humidity" ? [5, 4] : [],
        unit: cat.unit,
      });
    }
    return datasets;
  }

  function buildBands(): Band[] {
    const bands: Band[] = [];
    const bucketMs = activeBucketSeconds * 1000;
    for (const s of presenceSeries) {
      const readings = getCachedReadings(
        s.deviceId,
        activeBucketSeconds,
        activeRange.startSec,
        activeRange.endSec,
      );
      const fill = `${s.color}2e`; // ~18% alpha "sun" band
      let open: Band | null = null;
      for (const r of readings) {
        const occupied = metricValue(r, "presence") === 1;
        const t = r.timestamp.getTime();
        if (occupied) {
          if (open && t - open.endMs <= bucketMs * 2) {
            open.endMs = t;
          } else {
            if (open) bands.push(open);
            open = { startMs: t - bucketMs / 2, endMs: t, color: fill };
          }
        } else if (open) {
          open.endMs = Math.max(open.endMs, open.startMs) + bucketMs / 2;
          bands.push(open);
          open = null;
        }
      }
      if (open) {
        open.endMs += bucketMs / 2;
        bands.push(open);
      }
    }
    return bands;
  }

  function buildScales() {
    const scales: any = {
      x: {
        type: "time",
        time: {
          tooltipFormat: "PPpp",
          displayFormats: chartTimeFormats(timeRange),
        },
        grid: { display: false },
        ticks: { maxRotation: 0, font: { size: 11 } },
      },
    };
    units.forEach((unit, idx) => {
      const axisId = idx === 0 ? "y" : `y${idx}`;
      const isTemp = unit === "°C";
      scales[axisId] = {
        type: "linear",
        position: idx === 0 ? "left" : "right",
        title: { display: true, text: unit, font: { size: 12 } },
        grid: {
          drawOnChartArea: idx === 0,
          color: (ctx: any) =>
            ctx.tick?.value === 0 && isTemp
              ? "rgba(107, 114, 128, 0.9)"
              : "rgba(0, 0, 0, 0.05)",
          lineWidth: (ctx: any) => (ctx.tick?.value === 0 && isTemp ? 2 : 1),
        },
        ticks: { font: { size: 11 } },
        ...(unit === "%" ? { min: 0, max: 100 } : {}),
      };
    });
    return scales;
  }

  function updateZoomRangeFromChart(sourceChart: Chart) {
    const xScale = sourceChart.scales?.x as any;
    if (!xScale) return;
    const { min, max } = xScale;
    if (typeof min !== "number" || typeof max !== "number") return;
    zoomRange = { startSec: Math.floor(min / 1000), endSec: Math.floor(max / 1000) };
  }
  function resetZoom() {
    chart?.resetZoom();
    zoomRange = null;
  }
  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    resetZoom();
  }

  function createChart() {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    chart = new Chart(ctx, {
      type: "line",
      data: { datasets: buildDatasets() },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: "nearest", intersect: false, axis: "x" },
        plugins: {
          legend: { display: false },
          tooltip: {
            enabled: true,
            callbacks: {
              title: (items) =>
                items.length && items[0].parsed.x !== null
                  ? formatDateTime(new Date(items[0].parsed.x))
                  : "",
              label: (item) => {
                const unit = (item.dataset as any).unit ?? "";
                const y = item.parsed.y;
                return `${item.dataset.label}: ${y}${unit ? ` ${unit}` : ""}`;
              },
            },
          },
          zoom: {
            pan: {
              enabled: true,
              mode: "x",
              modifierKey: "shift",
              onPanComplete: ({ chart }) => updateZoomRangeFromChart(chart),
            },
            zoom: {
              drag: { enabled: true },
              mode: "x",
              onZoomComplete: ({ chart }) => updateZoomRangeFromChart(chart),
            },
            limits: { x: { min: "original", max: "original" } },
          },
        },
        scales: buildScales(),
      },
    });
    (chart as any).$bands = buildBands();
  }

  function updateChart() {
    if (!chart) return;
    chart.data.datasets = buildDatasets();
    chart.options.scales = buildScales() as any;
    const xAxis = chart.options.scales?.x as any;
    if (xAxis?.time) xAxis.time.displayFormats = chartTimeFormats(timeRange);
    (chart as any).$bands = buildBands();
    chart.update("none");
  }

  // Structural signature — when it changes we rebuild the chart (scales differ);
  // otherwise we update data in place.
  const structureSig = $derived(
    JSON.stringify({
      s: series.map((s) => `${s.deviceId}:${s.metric}:${s.color}`),
      u: units,
    }),
  );
  let prevSig: string | undefined = $state(undefined);

  let previousTimeRange: string | undefined = $state(undefined);
  $effect(() => {
    if (previousTimeRange !== undefined && timeRange !== previousTimeRange) {
      zoomRange = null;
    }
    previousTimeRange = timeRange;
  });

  $effect(() => {
    if (series.length === 0) {
      if (chart) {
        chart.destroy();
        chart = null;
      }
      prevSig = undefined;
      return;
    }
    if (!canvas) return;

    if (!chart || structureSig !== prevSig) {
      if (chart) {
        chart.destroy();
        chart = null;
      }
      createChart();
      prevSig = structureSig;
    } else {
      // touch reactive deps so data updates re-run this effect
      void activeRange;
      void seriesByDevice;
      updateChart();
    }
  });

  // Refresh axis time labels when the clock preference changes.
  $effect(() => {
    void $hourCycle;
    if (chart) {
      const xAxis = chart.options.scales?.x as any;
      if (xAxis?.time) {
        xAxis.time.displayFormats = chartTimeFormats(timeRange);
        chart.update("none");
      }
    }
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  const anyData = $derived(
    lineSeries.some(
      (s) =>
        getCachedReadings(s.deviceId, activeBucketSeconds, activeRange.startSec, activeRange.endSec)
          .length > 0,
    ) ||
      presenceSeries.some(
        (s) =>
          getCachedReadings(s.deviceId, activeBucketSeconds, activeRange.startSec, activeRange.endSec)
            .length > 0,
      ),
  );
</script>

<div class="chart-container">
  {#if series.length === 0}
    <div class="no-data">
      <p>Aucune série sélectionnée. Choisissez des capteurs dans le panneau de gauche.</p>
    </div>
  {:else}
    <canvas bind:this={canvas} oncontextmenu={handleContextMenu}></canvas>
    {#if !anyData}
      <div class="no-data overlay">
        <p>Chargement des données…</p>
      </div>
    {/if}
  {/if}
</div>

<style>
  .chart-container {
    width: 100%;
    height: 100%;
    min-height: 320px;
    position: relative;
    background: transparent;
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
    padding: 1rem;
  }

  .overlay {
    position: absolute;
    inset: 0;
    background: rgba(255, 255, 255, 0.9);
  }
</style>
