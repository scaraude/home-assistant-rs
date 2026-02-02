<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { deviceStateMemory, sensorsMemory } from "../memory";
  import type { EnergySensorReading } from "../api/sensors";
  import type { DeviceInfo, DeviceState } from "../types/devices";
  import { graphConfig, RECOMMENDED_COLORS } from "../stores/graphConfig";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import UnifiedChart from "../graphs/UnifiedChart.svelte";
  import EditableDeviceName from "../devices/EditableDeviceName.svelte";
  import Gauge from "../design-system/Gauge.svelte";
  import { URI_FRONTEND } from "../shared/constant/URI";

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
  let timeRange = $state<TimeRange>("24h");

  // Get the latest reading for this energy meter
  const latestReading = $derived.by(() => {
    const reading = $sensorsMemory.latestByDevice[params.id] ?? null;
    if (!reading || reading.type !== "energy_meter") return null;
    return reading as EnergySensorReading;
  });

  const powerMagnitude = $derived.by(() => {
    if (latestReading?.power === undefined) return null;
    return Math.abs(latestReading.power);
  });

  // Power flow direction
  const isProducing = $derived((latestReading?.power ?? 0) < 0);

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

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = null;

    try {
      await sensorsMemory.ensureDevices();
      // Find device info from sensors memory - energy meters are sensors
      const deviceInfo = $sensorsMemory.devices.find(
        (d) => d.device_id === params.id,
      );

      if (!deviceInfo) {
        error = "Energy meter not found";
        loading = false;
        return;
      }

      device = deviceInfo;

      // Fetch device state (battery, link quality)
      const state = await deviceStateMemory.ensureDeviceState(params.id);
      if (state) {
        deviceState = state;
      }

      // Initialize graph config for this sensor
      graphConfig.initializeSensors([
        { deviceId: params.id, color: deviceInfo.color },
      ]);
      graphConfig.isolateSensor(params.id);

      void sensorsMemory.ensureRecentReadings(params.id, 24);
    } catch (err) {
      console.error("Failed to load energy meter data:", err);
      error =
        err instanceof Error ? err.message : "Failed to load energy meter data";
    } finally {
      loading = false;
    }
  }

  function handleTimeRangeChange(range: TimeRange) {
    timeRange = range;
    graphConfig.setTimeRange(range);
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
    push(URI_FRONTEND.FLOORPLAN);
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

  function formatPower(value: number | undefined): string {
    if (value === undefined) return "--";
    if (Math.abs(value) >= 1000) {
      return (value / 1000).toFixed(2);
    }
    return value.toFixed(0);
  }

  function formatPowerUnit(value: number | undefined): string {
    if (value === undefined) return "W";
    if (Math.abs(value) >= 1000) {
      return "kW";
    }
    return "W";
  }

  function formatEnergy(value: number | undefined): string {
    if (value === undefined) return "--";
    return value.toFixed(2);
  }

  function formatVoltage(value: number | undefined): string {
    if (value === undefined) return "--";
    return value.toFixed(1);
  }

  function formatCurrent(value: number | undefined): string {
    if (value === undefined) return "--";
    return value.toFixed(2);
  }

  function formatFrequency(value: number | undefined): string {
    if (value === undefined) return "--";
    return value.toFixed(1);
  }

  function formatPowerFactor(value: number | undefined): string {
    if (value === undefined) return "--";
    return value.toFixed(2);
  }
</script>

<div class="energy-detail-page">
  <!-- Ambient background -->
  <div class="ambient-bg"></div>
  <div class="grid-overlay"></div>

  <header class="page-header">
    <button class="back-button" onclick={goBack} type="button">
      <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
        <path
          d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"
        />
      </svg>
      <span>Back</span>
    </button>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="loading-ring">
        <div class="ring-segment"></div>
        <div class="ring-segment"></div>
        <div class="ring-segment"></div>
      </div>
      <p class="loading-text">Connecting to energy meter...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <div class="error-icon">
        <svg viewBox="0 0 24 24" fill="currentColor" width="48" height="48">
          <path
            d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
          />
        </svg>
      </div>
      <p class="error-text">{error}</p>
      <button class="retry-btn" onclick={loadData} type="button">
        Retry Connection
      </button>
    </div>
  {:else if device}
    <div class="content">
      <!-- Device Identity Section -->
      <section class="device-identity">
        <div class="identity-icon">
          <svg viewBox="0 0 24 24" fill="currentColor" width="32" height="32">
            <path
              d="M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.07-.12C8.48 10.94 10.42 7.54 13 3h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z"
            />
          </svg>
          <div class="power-pulse"></div>
        </div>

        <div class="identity-details">
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
                class="edit-trigger"
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

          <div class="identity-meta">
            <span class="device-id">{params.id}</span>
            <span class="meta-divider">•</span>
            <span class="last-seen">
              {formatLastSeen(
                deviceState?.last_seen || latestReading?.timestamp,
              )}
            </span>
          </div>
        </div>

        <div class="device-badges">
          {#if deviceState?.battery_level != null}
            <StatusBadge type="battery" value={deviceState.battery_level} />
          {/if}
          {#if deviceState?.link_quality != null}
            <StatusBadge type="signal" value={deviceState.link_quality} />
          {/if}
        </div>
      </section>

      <!-- Power Display Hero -->
      <section class="power-hero">
        <div class="power-gauge">
          <Gauge
            value={powerMagnitude}
            min={0}
            max={6000}
            tone={isProducing ? "good" : "info"}
            label=""
            size="lg"
            unit={powerMagnitude === null
              ? ""
              : formatPowerUnit(powerMagnitude)}
            format={(value) => formatPower(value)}
            style="--gauge-stroke: 12; --gauge-track: rgba(255, 255, 255, 0.12); --gauge-text: #f1f5f9; --gauge-subtle: rgba(255, 255, 255, 0.5);"
          >
            {#snippet center()}
              <div class="power-value">
                <span class="value-number"
                  >{formatPower(powerMagnitude ?? undefined)}</span
                >
                <span class="value-unit"
                  >{formatPowerUnit(powerMagnitude ?? undefined)}</span
                >
              </div>
            {/snippet}
          </Gauge>
          <div class="flow-indicator" class:producing={isProducing}>
            <span class="flow-arrow">{isProducing ? "↑" : "↓"}</span>
            <span class="flow-label"
              >{isProducing ? "PRODUCING" : "CONSUMING"}</span
            >
          </div>
        </div>
      </section>

      <!-- Live Metrics Grid -->
      <section class="metrics-section">
        <h2 class="section-header">
          <span class="header-icon">
            <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
              <path
                d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zM7 10h2v7H7zm4-3h2v10h-2zm4 6h2v4h-2z"
              />
            </svg>
          </span>
          Live Metrics
        </h2>

        <div class="metrics-grid">
          <div class="metric-card voltage">
            <div class="metric-icon">
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="20"
                height="20"
              >
                <path
                  d="M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.07-.12C8.48 10.94 10.42 7.54 13 3h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z"
                />
              </svg>
            </div>
            <div class="metric-data">
              <span class="metric-value"
                >{formatVoltage(latestReading?.voltage)}</span
              >
              <span class="metric-unit">V</span>
            </div>
            <span class="metric-label">Voltage</span>
          </div>

          <div class="metric-card current">
            <div class="metric-icon">
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="20"
                height="20"
              >
                <path
                  d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm.31-8.86c-1.77-.45-2.34-.94-2.34-1.67 0-.84.79-1.43 2.1-1.43 1.38 0 1.9.66 1.94 1.64h1.71c-.05-1.34-.87-2.57-2.49-2.97V5H10.9v1.69c-1.51.32-2.72 1.3-2.72 2.81 0 1.79 1.49 2.69 3.66 3.21 1.95.46 2.34 1.15 2.34 1.87 0 .53-.39 1.39-2.1 1.39-1.6 0-2.23-.72-2.32-1.64H8.04c.1 1.7 1.36 2.66 2.86 2.97V19h2.34v-1.67c1.52-.29 2.72-1.16 2.73-2.77-.01-2.2-1.9-2.96-3.66-3.42z"
                />
              </svg>
            </div>
            <div class="metric-data">
              <span class="metric-value"
                >{formatCurrent(latestReading?.current)}</span
              >
              <span class="metric-unit">A</span>
            </div>
            <span class="metric-label">Current</span>
          </div>

          <div class="metric-card frequency">
            <div class="metric-icon">
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="20"
                height="20"
              >
                <path
                  d="M3.5 18.49l6-6.01 4 4L22 6.92l-1.41-1.41-7.09 7.97-4-4L2 16.99z"
                />
              </svg>
            </div>
            <div class="metric-data">
              <span class="metric-value"
                >{formatFrequency(latestReading?.ac_frequency)}</span
              >
              <span class="metric-unit">Hz</span>
            </div>
            <span class="metric-label">AC Frequency</span>
          </div>

          <div class="metric-card power-factor">
            <div class="metric-icon">
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="20"
                height="20"
              >
                <path
                  d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1.41 16.09V20h-2.67v-1.93c-1.71-.36-3.16-1.46-3.27-3.4h1.96c.1 1.05.82 1.87 2.65 1.87 1.96 0 2.4-.98 2.4-1.59 0-.83-.44-1.61-2.67-2.14-2.48-.6-4.18-1.62-4.18-3.67 0-1.72 1.39-2.84 3.11-3.21V4h2.67v1.95c1.86.45 2.79 1.86 2.85 3.39H14.3c-.05-1.11-.64-1.87-2.22-1.87-1.5 0-2.4.68-2.4 1.64 0 .84.65 1.39 2.67 1.91s4.18 1.39 4.18 3.91c-.01 1.83-1.38 2.83-3.12 3.16z"
                />
              </svg>
            </div>
            <div class="metric-data">
              <span class="metric-value"
                >{formatPowerFactor(latestReading?.power_factor)}</span
              >
              <span class="metric-unit">PF</span>
            </div>
            <span class="metric-label">Power Factor</span>
          </div>
        </div>
      </section>

      <!-- Energy Totals -->
      <section class="energy-totals">
        <h2 class="section-header">
          <span class="header-icon">
            <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
              <path
                d="M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.07-.12C8.48 10.94 10.42 7.54 13 3h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z"
              />
            </svg>
          </span>
          Energy Totals
        </h2>

        <div class="energy-cards">
          <div class="energy-card consumed">
            <div class="energy-visual">
              <div class="energy-bar">
                <div class="bar-fill"></div>
              </div>
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="24"
                height="24"
                class="energy-icon"
              >
                <path
                  d="M16 6l2.29 2.29-4.88 4.88-4-4L2 16.59 3.41 18l6-6 4 4 6.3-6.29L22 12V6z"
                />
              </svg>
            </div>
            <div class="energy-info">
              <span class="energy-label">Consumed</span>
              <div class="energy-value">
                <span class="value">{formatEnergy(latestReading?.energy)}</span>
                <span class="unit">kWh</span>
              </div>
            </div>
          </div>

          <div class="energy-card produced">
            <div class="energy-visual">
              <div class="energy-bar">
                <div class="bar-fill"></div>
              </div>
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="24"
                height="24"
                class="energy-icon"
              >
                <path
                  d="M16 18l2.29-2.29-4.88-4.88-4 4L2 7.41 3.41 6l6 6 4-4 6.3 6.29L22 12v6z"
                />
              </svg>
            </div>
            <div class="energy-info">
              <span class="energy-label">Produced</span>
              <div class="energy-value">
                <span class="value"
                  >{formatEnergy(latestReading?.produced_energy)}</span
                >
                <span class="unit">kWh</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Chart Section -->
      <section class="chart-section">
        <div class="chart-header">
          <h2 class="section-header">
            <span class="header-icon">
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                width="18"
                height="18"
              >
                <path
                  d="M3.5 18.49l6-6.01 4 4L22 6.92l-1.41-1.41-7.09 7.97-4-4L2 16.99z"
                />
              </svg>
            </span>
            Power History
          </h2>

          <div class="time-range-selector">
            {#each ["24h", "1w", "1m", "1y"] as range}
              <button
                class="range-btn"
                class:active={timeRange === range}
                onclick={() => handleTimeRangeChange(range as TimeRange)}
                type="button"
              >
                {range}
              </button>
            {/each}
          </div>
        </div>

        <div class="chart-container">
          <UnifiedChart sensors={sensorConfig} {timeRange} metricType="power" />
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  .energy-detail-page {
    position: relative;
    min-height: 100vh;
    background: linear-gradient(135deg, #0f172a 0%, #1e293b 50%, #0f172a 100%);
    padding: 1.5rem;
    overflow: hidden;
  }

  /* Ambient background effects */
  .ambient-bg {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: radial-gradient(
        ellipse 80% 50% at 50% -20%,
        rgba(6, 182, 212, 0.15),
        transparent
      ),
      radial-gradient(
        ellipse 60% 40% at 80% 100%,
        rgba(59, 130, 246, 0.1),
        transparent
      );
    pointer-events: none;
    z-index: 0;
  }

  .grid-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-image: linear-gradient(
        rgba(6, 182, 212, 0.03) 1px,
        transparent 1px
      ),
      linear-gradient(90deg, rgba(6, 182, 212, 0.03) 1px, transparent 1px);
    background-size: 40px 40px;
    pointer-events: none;
    z-index: 0;
  }

  .page-header {
    position: relative;
    z-index: 1;
    margin-bottom: 1.5rem;
  }

  .back-button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8125rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    cursor: pointer;
    transition: all 0.2s ease;
    backdrop-filter: blur(8px);
  }

  .back-button:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(6, 182, 212, 0.3);
    color: #06b6d4;
  }

  /* Loading state */
  .loading-state {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 6rem 2rem;
  }

  .loading-ring {
    position: relative;
    width: 80px;
    height: 80px;
    margin-bottom: 1.5rem;
  }

  .ring-segment {
    position: absolute;
    width: 100%;
    height: 100%;
    border: 3px solid transparent;
    border-top-color: #06b6d4;
    border-radius: 50%;
    animation: spin 1.2s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  }

  .ring-segment:nth-child(2) {
    animation-delay: -0.4s;
    border-top-color: #0ea5e9;
  }

  .ring-segment:nth-child(3) {
    animation-delay: -0.8s;
    border-top-color: #3b82f6;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .loading-text {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.5);
    letter-spacing: 0.05em;
  }

  /* Error state */
  .error-state {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4rem 2rem;
    text-align: center;
  }

  .error-icon {
    color: #ef4444;
    margin-bottom: 1rem;
    opacity: 0.8;
  }

  .error-text {
    font-size: 1rem;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 1.5rem;
  }

  .retry-btn {
    padding: 0.75rem 1.5rem;
    background: linear-gradient(135deg, #06b6d4, #0ea5e9);
    border: none;
    border-radius: 8px;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.875rem;
    font-weight: 600;
    color: white;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .retry-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 20px rgba(6, 182, 212, 0.3);
  }

  /* Content */
  .content {
    position: relative;
    z-index: 1;
    max-width: 900px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* Device Identity */
  .device-identity {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.25rem 1.5rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    backdrop-filter: blur(12px);
  }

  .identity-icon {
    position: relative;
    width: 56px;
    height: 56px;
    background: linear-gradient(
      135deg,
      rgba(6, 182, 212, 0.2),
      rgba(59, 130, 246, 0.2)
    );
    border: 1px solid rgba(6, 182, 212, 0.3);
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #06b6d4;
    flex-shrink: 0;
  }

  .power-pulse {
    position: absolute;
    inset: -4px;
    border: 2px solid rgba(6, 182, 212, 0.4);
    border-radius: 18px;
    animation: pulse-ring 2s ease-out infinite;
  }

  @keyframes pulse-ring {
    0% {
      transform: scale(1);
      opacity: 1;
    }
    100% {
      transform: scale(1.2);
      opacity: 0;
    }
  }

  .identity-details {
    flex: 1;
    min-width: 0;
  }

  .device-name {
    font-family:
      "SF Pro Display",
      -apple-system,
      sans-serif;
    font-size: 1.375rem;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0 0 0.25rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .edit-trigger {
    padding: 0.375rem;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.3);
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .edit-trigger:hover {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.7);
  }

  .identity-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: rgba(255, 255, 255, 0.4);
  }

  .device-id {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.75rem;
  }

  .meta-divider {
    opacity: 0.3;
  }

  .device-badges {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  :global(.device-name-text) {
    font-size: 1.5rem;
    font-weight: 700;
    color: #f8fafc;
  }

  /* Power Hero Section */
  .power-hero {
    display: flex;
    justify-content: center;
    padding: 2rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 20px;
    backdrop-filter: blur(12px);
  }

  .power-gauge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
  }

  .power-gauge :global(.gauge-center) {
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.5rem;
    padding: 0 1rem;
  }

  .flow-indicator {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    background: rgba(6, 182, 212, 0.15);
    border: 1px solid rgba(6, 182, 212, 0.3);
    border-radius: 20px;
    margin-bottom: 0.75rem;
  }

  .flow-indicator.producing {
    background: rgba(34, 197, 94, 0.15);
    border-color: rgba(34, 197, 94, 0.3);
  }

  .flow-arrow {
    font-size: 0.875rem;
    color: #06b6d4;
    animation: flow-bounce 1.5s ease-in-out infinite;
  }

  .flow-indicator.producing .flow-arrow {
    color: #22c55e;
  }

  @keyframes flow-bounce {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-2px);
    }
  }

  .flow-label {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.625rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    color: #06b6d4;
  }

  .flow-indicator.producing .flow-label {
    color: #22c55e;
  }

  .power-value {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
  }

  .value-number {
    font-family:
      "SF Pro Display",
      -apple-system,
      sans-serif;
    font-size: 3rem;
    font-weight: 700;
    color: #f1f5f9;
    letter-spacing: -0.02em;
    line-height: 1;
  }

  .value-unit {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 1.25rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.5);
  }

  .power-gauge :global(.gauge-label) {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.6875rem;
    color: rgba(255, 255, 255, 0.35);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  /* Metrics Section */
  .metrics-section,
  .energy-totals,
  .chart-section {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    backdrop-filter: blur(12px);
    padding: 1.5rem;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.8125rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    margin: 0 0 1.25rem;
  }

  .header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: rgba(6, 182, 212, 0.15);
    border-radius: 6px;
    color: #06b6d4;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }

  .metric-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 1.25rem 1rem;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    text-align: center;
    transition: all 0.2s ease;
  }

  .metric-card:hover {
    background: rgba(0, 0, 0, 0.3);
    border-color: rgba(6, 182, 212, 0.2);
  }

  .metric-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    color: white;
  }

  .metric-card.voltage .metric-icon {
    background: linear-gradient(
      135deg,
      rgba(139, 92, 246, 0.3),
      rgba(139, 92, 246, 0.1)
    );
    color: #a78bfa;
  }

  .metric-card.current .metric-icon {
    background: linear-gradient(
      135deg,
      rgba(34, 197, 94, 0.3),
      rgba(34, 197, 94, 0.1)
    );
    color: #4ade80;
  }

  .metric-card.frequency .metric-icon {
    background: linear-gradient(
      135deg,
      rgba(251, 191, 36, 0.3),
      rgba(251, 191, 36, 0.1)
    );
    color: #fbbf24;
  }

  .metric-card.power-factor .metric-icon {
    background: linear-gradient(
      135deg,
      rgba(59, 130, 246, 0.3),
      rgba(59, 130, 246, 0.1)
    );
    color: #60a5fa;
  }

  .metric-data {
    display: flex;
    align-items: baseline;
    gap: 0.125rem;
  }

  .metric-value {
    font-family:
      "SF Pro Display",
      -apple-system,
      sans-serif;
    font-size: 1.5rem;
    font-weight: 700;
    color: #f1f5f9;
    letter-spacing: -0.01em;
  }

  .metric-unit {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.4);
  }

  .metric-label {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.4);
  }

  /* Energy Totals */
  .energy-cards {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .energy-card {
    display: flex;
    gap: 1rem;
    padding: 1.25rem;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
  }

  .energy-visual {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    flex-shrink: 0;
  }

  .energy-bar {
    position: absolute;
    inset: 0;
    border-radius: 12px;
    overflow: hidden;
  }

  .energy-card.consumed .energy-bar {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid rgba(6, 182, 212, 0.2);
  }

  .energy-card.produced .energy-bar {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .bar-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 60%;
    transition: height 0.5s ease;
  }

  .energy-card.consumed .bar-fill {
    background: linear-gradient(
      to top,
      rgba(6, 182, 212, 0.4),
      rgba(6, 182, 212, 0.1)
    );
  }

  .energy-card.produced .bar-fill {
    background: linear-gradient(
      to top,
      rgba(34, 197, 94, 0.4),
      rgba(34, 197, 94, 0.1)
    );
  }

  .energy-icon {
    position: relative;
    z-index: 1;
  }

  .energy-card.consumed .energy-icon {
    color: #06b6d4;
  }

  .energy-card.produced .energy-icon {
    color: #22c55e;
  }

  .energy-info {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .energy-label {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.4);
    margin-bottom: 0.25rem;
  }

  .energy-value {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
  }

  .energy-value .value {
    font-family:
      "SF Pro Display",
      -apple-system,
      sans-serif;
    font-size: 1.5rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .energy-value .unit {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.4);
  }

  /* Chart Section */
  .chart-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .chart-header .section-header {
    margin: 0;
  }

  .time-range-selector {
    display: flex;
    gap: 0.25rem;
    padding: 0.25rem;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
  }

  .range-btn {
    padding: 0.5rem 0.875rem;
    background: transparent;
    border: none;
    border-radius: 6px;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.75rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .range-btn:hover {
    color: rgba(255, 255, 255, 0.8);
    background: rgba(255, 255, 255, 0.05);
  }

  .range-btn.active {
    background: #06b6d4;
    color: white;
    box-shadow: 0 2px 8px rgba(6, 182, 212, 0.3);
  }

  .chart-container {
    height: 400px;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 12px;
    overflow: hidden;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .energy-detail-page {
      padding: 1rem;
    }

    .device-identity {
      flex-wrap: wrap;
    }

    .device-badges {
      width: 100%;
      justify-content: flex-start;
      margin-top: 0.5rem;
    }

    .power-gauge {
      width: 200px;
      height: 200px;
    }

    .value-number {
      font-size: 2.5rem;
    }

    .metrics-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .energy-cards {
      grid-template-columns: 1fr;
    }

    .chart-header {
      flex-direction: column;
      align-items: stretch;
    }

    .time-range-selector {
      justify-content: center;
    }

    .chart-container {
      height: 300px;
    }
  }

  @media (max-width: 480px) {
    .power-hero {
      padding: 1.5rem 1rem;
    }

    .power-gauge {
      width: 180px;
      height: 180px;
    }

    .value-number {
      font-size: 2rem;
    }

    .metrics-grid {
      grid-template-columns: 1fr;
    }

    .metric-card {
      flex-direction: row;
      text-align: left;
      padding: 1rem;
    }

    .metric-icon {
      flex-shrink: 0;
    }

    .chart-container {
      height: 250px;
    }
  }
</style>
