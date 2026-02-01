<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { deviceStateMemory, sensorsMemory } from "../memory";
  import type { TempHumiditySensorReading } from "../api/sensors";
  import type { DeviceInfo, DeviceState } from "../types/devices";
  import { graphConfig, RECOMMENDED_COLORS } from "../stores/graphConfig";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import UnifiedChart from "../graphs/UnifiedChart.svelte";
  import Icon from "../design-system/Icon.svelte";
  import EditableDeviceName from "../devices/EditableDeviceName.svelte";

  interface Props {
    params: { id: string };
  }

  let { params }: Props = $props();

  let device = $state<DeviceInfo | null>(null);
  let deviceState = $state<DeviceState | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let editingName = $state(false);

  // Time range state (local to this page)
  type TimeRange = "24h" | "1w" | "1m" | "1y";
  type Metric = "temperature" | "humidity" | "both";
  let timeRange = $state<TimeRange>("24h");
  let metric = $state<Metric>("both");

  // Get the latest reading for this sensor
  const latestReading = $derived.by(() => {
    const reading = $sensorsMemory.latestByDevice[params.id] ?? null;
    if (!reading || reading.type !== "temp_humidity") return null;
    return reading as TempHumiditySensorReading;
  });

  // Get sensor config for the chart
  const sensorConfig = $derived([
    {
      deviceId: params.id,
      color: device?.color || RECOMMENDED_COLORS[0],
      visible: true,
    },
  ]);

  const deviceName = $derived.by(() => {
    const memoryDevice = $sensorsMemory.devices.find(
      (d) => d.device_id === params.id,
    );
    return memoryDevice?.name ?? device?.name ?? params.id;
  });

  // Calculate comfort level based on temperature and humidity
  const comfortLevel = $derived.by(() => {
    if (!latestReading) return null;
    const temp = latestReading.temperature;
    const hum = latestReading.humidity;

    // Ideal: 20-24°C, 40-60% humidity
    const tempScore =
      temp >= 20 && temp <= 24 ? 100 : temp >= 18 && temp <= 26 ? 70 : 40;
    const humScore =
      hum >= 40 && hum <= 60 ? 100 : hum >= 30 && hum <= 70 ? 70 : 40;

    const score = (tempScore + humScore) / 2;
    if (score >= 85)
      return { label: "Optimal", color: "var(--accent-success)" };
    if (score >= 60) return { label: "Good", color: "var(--accent-warning)" };
    return { label: "Check", color: "var(--accent-danger)" };
  });

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = null;

    try {
      await sensorsMemory.ensureDevices();
      const deviceInfo = $sensorsMemory.devices.find(
        (d) => d.device_id === params.id,
      );

      if (!deviceInfo) {
        error = "Sensor not found";
        loading = false;
        return;
      }

      device = deviceInfo;

      const state = await deviceStateMemory.ensureDeviceState(params.id);
      if (state) {
        deviceState = state;
      }

      graphConfig.initializeSensors([
        { deviceId: params.id, color: deviceInfo.color },
      ]);
      graphConfig.isolateSensor(params.id);

      void sensorsMemory.ensureRecentReadings(params.id, 24);
    } catch (err) {
      console.error("Failed to load sensor data:", err);
      error = err instanceof Error ? err.message : "Failed to load sensor data";
    } finally {
      loading = false;
    }
  }

  function handleTimeRangeChange(range: TimeRange) {
    timeRange = range;
    graphConfig.setTimeRange(range);
  }

  function handleMetricChange(m: Metric) {
    metric = m;
    graphConfig.setMetric(m);
  }

  function startEditingName() {
    editingName = true;
  }

  function handleNameSaved() {
    editingName = false;
    const memoryDevice = $sensorsMemory.devices.find(
      (d) => d.device_id === params.id,
    );
    if (memoryDevice) {
      device = memoryDevice;
    }
  }

  function goBack() {
    push("/network");
  }

  function formatLastSeen(date: Date | null | undefined): string {
    if (!date) return "Unknown";
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (seconds < 60) return "Just now";
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return date.toLocaleDateString();
  }
</script>

<div class="detail-page">
  <!-- Ambient background gradient -->
  <div class="ambient-bg"></div>

  <header class="page-header">
    <button class="back-btn" onclick={goBack} type="button">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <path d="M19 12H5M12 19l-7-7 7-7" />
      </svg>
      <span>Back</span>
    </button>

    <div class="header-badges">
      {#if deviceState?.battery_level != null}
        <StatusBadge type="battery" value={deviceState.battery_level} />
      {/if}
      {#if deviceState?.link_quality != null}
        <StatusBadge type="signal" value={deviceState.link_quality} />
      {/if}
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="loader">
        <div class="loader-ring"></div>
        <div class="loader-ring"></div>
        <div class="loader-ring"></div>
      </div>
      <p>Loading sensor data...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <div class="error-icon">
        <Icon name="error" size={48} />
      </div>
      <p>{error}</p>
      <button class="btn-primary" onclick={loadData} type="button">
        Retry
      </button>
    </div>
  {:else if device}
    <div class="content">
      <!-- Hero Section -->
      <section class="hero-section">
        <div class="device-identity">
          <div class="device-icon-wrapper">
            <div class="device-icon">
              <Icon name="thermometer" size={32} />
            </div>
            <div class="pulse-ring"></div>
          </div>

          <div class="device-meta">
            <h1 class="device-name">
              <EditableDeviceName
                deviceId={params.id}
                name={deviceName}
                isEdit={editingName}
                onSaved={handleNameSaved}
                class="device-name-text"
              />
              {#if !editingName}
                <button
                  class="edit-btn"
                  onclick={startEditingName}
                  title="Edit name"
                  type="button"
                >
                  <svg
                    viewBox="0 0 24 24"
                    fill="currentColor"
                    width="16"
                    height="16"
                  >
                    <path
                      d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"
                    />
                  </svg>
                </button>
              {/if}
            </h1>
            <p class="device-id">{params.id}</p>
            <p class="last-seen">
              Last seen: {formatLastSeen(
                deviceState?.last_seen || latestReading?.timestamp,
              )}
            </p>
          </div>
        </div>

        <!-- Large Metrics Display -->
        <div class="metrics-hero">
          <div class="metric-gauge temperature">
            <div class="gauge-ring">
              <svg viewBox="0 0 120 120">
                <circle cx="60" cy="60" r="54" class="gauge-bg" />
                <circle
                  cx="60"
                  cy="60"
                  r="54"
                  class="gauge-fill"
                  style="--progress: {latestReading
                    ? Math.min((latestReading.temperature + 10) / 50, 1)
                    : 0}"
                />
              </svg>
            </div>
            <div class="gauge-content">
              <span class="gauge-value">
                {latestReading ? latestReading.temperature.toFixed(1) : "--"}
              </span>
              <span class="gauge-unit">°C</span>
            </div>
            <span class="gauge-label">Temperature</span>
          </div>

          <div class="metric-gauge humidity">
            <div class="gauge-ring">
              <svg viewBox="0 0 120 120">
                <circle cx="60" cy="60" r="54" class="gauge-bg" />
                <circle
                  cx="60"
                  cy="60"
                  r="54"
                  class="gauge-fill"
                  style="--progress: {latestReading
                    ? latestReading.humidity / 100
                    : 0}"
                />
              </svg>
            </div>
            <div class="gauge-content">
              <span class="gauge-value">
                {latestReading ? latestReading.humidity.toFixed(0) : "--"}
              </span>
              <span class="gauge-unit">%</span>
            </div>
            <span class="gauge-label">Humidity</span>
          </div>

          {#if comfortLevel}
            <div
              class="comfort-indicator"
              style="--comfort-color: {comfortLevel.color}"
            >
              <div class="comfort-dot"></div>
              <span class="comfort-label">{comfortLevel.label}</span>
              <span class="comfort-sublabel">Comfort</span>
            </div>
          {/if}
        </div>
      </section>

      <!-- Chart Section -->
      <section class="chart-section">
        <div class="section-header">
          <h2>Historical Data</h2>
          <div class="toolbar">
            <div class="btn-group">
              <button
                class="toolbar-btn"
                class:active={metric === "temperature"}
                onclick={() => handleMetricChange("temperature")}
                type="button"
              >
                Temp
              </button>
              <button
                class="toolbar-btn"
                class:active={metric === "humidity"}
                onclick={() => handleMetricChange("humidity")}
                type="button"
              >
                Humidity
              </button>
              <button
                class="toolbar-btn"
                class:active={metric === "both"}
                onclick={() => handleMetricChange("both")}
                type="button"
              >
                Both
              </button>
            </div>

            <div class="toolbar-divider"></div>

            <div class="btn-group">
              <button
                class="toolbar-btn"
                class:active={timeRange === "24h"}
                onclick={() => handleTimeRangeChange("24h")}
                type="button"
              >
                24h
              </button>
              <button
                class="toolbar-btn"
                class:active={timeRange === "1w"}
                onclick={() => handleTimeRangeChange("1w")}
                type="button"
              >
                1w
              </button>
              <button
                class="toolbar-btn"
                class:active={timeRange === "1m"}
                onclick={() => handleTimeRangeChange("1m")}
                type="button"
              >
                1m
              </button>
              <button
                class="toolbar-btn"
                class:active={timeRange === "1y"}
                onclick={() => handleTimeRangeChange("1y")}
                type="button"
              >
                1y
              </button>
            </div>
          </div>
        </div>

        <div class="chart-container">
          <UnifiedChart
            sensors={sensorConfig}
            {metric}
            {timeRange}
            metricType="temperature"
          />
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  /* === Design Tokens === */
  .detail-page {
    --accent-primary: #f59e0b;
    --accent-secondary: #3b82f6;
    --accent-success: #10b981;
    --accent-warning: #f59e0b;
    --accent-danger: #ef4444;
    --surface-elevated: rgba(255, 255, 255, 0.98);
    --surface-glass: rgba(255, 255, 255, 0.7);
    --text-primary: #0f172a;
    --text-secondary: #475569;
    --text-muted: #94a3b8;
    --border-subtle: rgba(0, 0, 0, 0.06);
    --temp-gradient: linear-gradient(135deg, #ef4444, #f97316, #eab308);
    --humidity-gradient: linear-gradient(135deg, #06b6d4, #3b82f6, #8b5cf6);
  }

  .detail-page {
    position: relative;
    min-height: 100vh;
    background: linear-gradient(135deg, #f8fafc 0%, #f1f5f9 50%, #e2e8f0 100%);
    padding: var(--space-4);
    overflow-x: hidden;
  }

  /* Ambient background effect */
  .ambient-bg {
    position: fixed;
    top: -50%;
    right: -30%;
    width: 80vw;
    height: 80vw;
    background: radial-gradient(
      circle,
      rgba(245, 158, 11, 0.08) 0%,
      transparent 70%
    );
    pointer-events: none;
    z-index: 0;
  }

  /* === Header === */
  .page-header {
    position: relative;
    z-index: 10;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-6);
    max-width: 1000px;
    margin-left: auto;
    margin-right: auto;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  }

  .back-btn:hover {
    background: white;
    color: var(--text-primary);
    transform: translateX(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  }

  .back-btn svg {
    width: 18px;
    height: 18px;
  }

  .header-badges {
    display: flex;
    gap: var(--space-2);
  }

  /* === Loading State === */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    gap: var(--space-4);
  }

  .loader {
    position: relative;
    width: 60px;
    height: 60px;
  }

  .loader-ring {
    position: absolute;
    inset: 0;
    border: 3px solid transparent;
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1.2s ease-in-out infinite;
  }

  .loader-ring:nth-child(2) {
    inset: 8px;
    border-top-color: var(--accent-secondary);
    animation-delay: 0.15s;
  }

  .loader-ring:nth-child(3) {
    inset: 16px;
    border-top-color: var(--accent-success);
    animation-delay: 0.3s;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading-state p {
    font-size: 0.9375rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  /* === Error State === */
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    gap: var(--space-4);
    text-align: center;
  }

  .error-icon {
    width: 80px;
    height: 80px;
    background: linear-gradient(135deg, #fef2f2, #fee2e2);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-danger);
  }

  .error-state p {
    font-size: 1rem;
    color: var(--text-secondary);
  }

  /* === Content === */
  .content {
    position: relative;
    z-index: 5;
    max-width: 1000px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* === Hero Section === */
  .hero-section {
    background: var(--surface-elevated);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 4px 24px rgba(0, 0, 0, 0.04);
    border: 1px solid var(--border-subtle);
  }

  .device-identity {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
    padding-bottom: var(--space-5);
    border-bottom: 1px solid var(--border-subtle);
  }

  .device-icon-wrapper {
    position: relative;
    flex-shrink: 0;
  }

  .device-icon {
    width: 64px;
    height: 64px;
    background: var(--temp-gradient);
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    box-shadow: 0 4px 16px rgba(245, 158, 11, 0.3);
  }

  .pulse-ring {
    position: absolute;
    inset: -4px;
    border: 2px solid rgba(245, 158, 11, 0.3);
    border-radius: 20px;
    animation: pulse-ring 2s ease-out infinite;
  }

  @keyframes pulse-ring {
    0% {
      transform: scale(1);
      opacity: 1;
    }
    100% {
      transform: scale(1.15);
      opacity: 0;
    }
  }

  .device-meta {
    flex: 1;
    min-width: 0;
  }

  .device-name {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 var(--space-1);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    letter-spacing: -0.02em;
  }

  .edit-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: var(--space-1);
    opacity: 0.4;
    transition: opacity 0.2s;
    display: flex;
    align-items: center;
    color: var(--text-secondary);
  }

  .edit-btn:hover {
    opacity: 1;
  }

  .device-id {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0 0 var(--space-1);
  }

  .last-seen {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin: 0;
  }

  :global(.device-name-text) {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  /* === Metrics Hero === */
  .metrics-hero {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: var(--space-4);
    align-items: center;
  }

  .metric-gauge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }

  .gauge-ring {
    position: relative;
    width: 120px;
    height: 120px;
  }

  .gauge-ring svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }

  .gauge-bg {
    fill: none;
    stroke: #e2e8f0;
    stroke-width: 8;
  }

  .gauge-fill {
    fill: none;
    stroke-width: 8;
    stroke-linecap: round;
    stroke-dasharray: 339.292;
    stroke-dashoffset: calc(339.292 * (1 - var(--progress, 0)));
    transition: stroke-dashoffset 1s ease-out;
  }

  .temperature .gauge-fill {
    stroke: url(#temp-gradient);
    stroke: #f59e0b;
  }

  .humidity .gauge-fill {
    stroke: url(#humidity-gradient);
    stroke: #3b82f6;
  }

  .gauge-content {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
  }

  .gauge-value {
    font-size: 1.75rem;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }

  .gauge-unit {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-muted);
    align-self: flex-start;
    margin-top: 8px;
  }

  .gauge-label {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Comfort Indicator */
  .comfort-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
    background: linear-gradient(135deg, #f8fafc, #f1f5f9);
    border-radius: var(--radius-lg);
    gap: var(--space-2);
  }

  .comfort-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--comfort-color);
    box-shadow: 0 0 20px var(--comfort-color);
    animation: comfort-pulse 2s ease-in-out infinite;
  }

  @keyframes comfort-pulse {
    0%,
    100% {
      box-shadow: 0 0 12px var(--comfort-color);
    }
    50% {
      box-shadow: 0 0 24px var(--comfort-color);
    }
  }

  .comfort-label {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .comfort-sublabel {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  /* === Chart Section === */
  .chart-section {
    background: var(--surface-elevated);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 4px 24px rgba(0, 0, 0, 0.04);
    border: 1px solid var(--border-subtle);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-4);
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .section-header h2 {
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
    letter-spacing: -0.01em;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .btn-group {
    display: flex;
    background: #f1f5f9;
    padding: 3px;
    border-radius: var(--radius-md);
    gap: 2px;
  }

  .toolbar-btn {
    padding: var(--space-1) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toolbar-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.5);
  }

  .toolbar-btn.active {
    background: white;
    color: var(--text-primary);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }

  .toolbar-divider {
    width: 1px;
    height: 20px;
    background: #e2e8f0;
  }

  .chart-container {
    height: 400px;
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  /* === Buttons === */
  .btn-primary {
    padding: var(--space-3) var(--space-5);
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: 0.9375rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-primary:hover {
    background: #d97706;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(245, 158, 11, 0.3);
  }

  /* === Responsive === */
  @media (max-width: 640px) {
    .detail-page {
      padding: var(--space-3);
    }

    .hero-section {
      padding: var(--space-4);
    }

    .device-identity {
      flex-direction: column;
      text-align: center;
    }

    .device-name {
      justify-content: center;
      font-size: 1.25rem;
    }

    .metrics-hero {
      grid-template-columns: repeat(2, 1fr);
    }

    .gauge-ring {
      width: 100px;
      height: 100px;
    }

    .gauge-value {
      font-size: 1.5rem;
    }

    .section-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .toolbar {
      width: 100%;
    }

    .btn-group {
      flex: 1;
    }

    .toolbar-btn {
      flex: 1;
      text-align: center;
      padding: var(--space-2) var(--space-2);
    }

    .toolbar-divider {
      display: none;
    }

    .chart-container {
      height: 300px;
    }

    .comfort-indicator {
      grid-column: span 2;
    }
  }
</style>
