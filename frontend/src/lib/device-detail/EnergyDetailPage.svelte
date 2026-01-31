<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { devicesMemory, deviceStateMemory, sensorsMemory } from "../memory";
  import type { EnergySensorReading } from "../api/sensors";
  import type { DeviceInfo, DeviceState } from "../types/devices";
  import {
    graphConfig,
    RECOMMENDED_COLORS,
  } from "../stores/graphConfig";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import UnifiedChart from "../graphs/UnifiedChart.svelte";

  interface Props {
    params: { id: string };
  }

  let { params }: Props = $props();

  let device = $state<DeviceInfo | null>(null);
  let deviceState = $state<DeviceState | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let editingName = $state(false);
  let editedName = $state("");
  let savingName = $state(false);

  // Time range state (local to this page)
  type TimeRange = "24h" | "1w" | "1m" | "1y";
  let timeRange = $state<TimeRange>("24h");

  // Get the latest reading for this energy meter
  const latestReading = $derived.by(() => {
    const reading = $sensorsMemory.latestByDevice[params.id] ?? null;
    if (!reading || reading.type !== "energy_meter") return null;
    return reading as EnergySensorReading;
  });

  // Get sensor config for the chart
  const sensorConfig = $derived([
    {
      deviceId: params.id,
      color: device?.color || RECOMMENDED_COLORS[0],
      visible: true,
    },
  ]);

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
      editedName = deviceInfo.name;

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
    editedName = device?.name || "";
    editingName = true;
  }

  function cancelEditingName() {
    editingName = false;
    editedName = device?.name || "";
  }

  async function saveName() {
    if (!device || editedName.trim() === device.name) {
      editingName = false;
      return;
    }

    savingName = true;
    try {
      await devicesMemory.setDeviceName(params.id, editedName.trim());
      device = { ...device, name: editedName.trim() };
      editingName = false;
    } catch (err) {
      console.error("Failed to save name:", err);
    } finally {
      savingName = false;
    }
  }

  function handleNameKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      void saveName();
    } else if (event.key === "Escape") {
      cancelEditingName();
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

  function formatPower(value: number | undefined): string {
    if (value === undefined) return "--";
    if (Math.abs(value) >= 1000) {
      return `${(value / 1000).toFixed(2)} kW`;
    }
    return `${value.toFixed(0)} W`;
  }

  function formatEnergy(value: number | undefined): string {
    if (value === undefined) return "--";
    return `${value.toFixed(2)} kWh`;
  }

  function formatVoltage(value: number | undefined): string {
    if (value === undefined) return "--";
    return `${value.toFixed(1)} V`;
  }

  function formatCurrent(value: number | undefined): string {
    if (value === undefined) return "--";
    return `${value.toFixed(2)} A`;
  }

  function formatFrequency(value: number | undefined): string {
    if (value === undefined) return "--";
    return `${value.toFixed(1)} Hz`;
  }

  function formatPowerFactor(value: number | undefined): string {
    if (value === undefined) return "--";
    return value.toFixed(2);
  }
</script>

<div class="energy-detail-page">
  <header class="page-header">
    <button class="back-button" onclick={goBack} type="button">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
        width="20"
        height="20"
      >
        <path
          d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"
        />
      </svg>
      Back
    </button>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Loading energy meter data...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <p>{error}</p>
      <button class="btn btn-primary" onclick={loadData} type="button">
        Retry
      </button>
    </div>
  {:else if device}
    <div class="content">
      <div class="device-header">
        <div class="device-icon">
          <span class="icon-emoji">⚡</span>
        </div>
        <div class="device-info">
          {#if editingName}
            <div class="name-edit">
              <input
                type="text"
                bind:value={editedName}
                onkeydown={handleNameKeydown}
                class="name-input"
                disabled={savingName}
              />
              <button
                class="btn btn-sm btn-primary"
                onclick={saveName}
                disabled={savingName}
                type="button"
              >
                {savingName ? "Saving..." : "Save"}
              </button>
              <button
                class="btn btn-sm btn-secondary"
                onclick={cancelEditingName}
                disabled={savingName}
                type="button"
              >
                Cancel
              </button>
            </div>
          {:else}
            <h1 class="device-name">
              {device.name}
              <button
                class="edit-name-btn"
                onclick={startEditingName}
                title="Edit name"
                type="button"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path
                    d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"
                  />
                </svg>
              </button>
            </h1>
          {/if}
          <p class="device-id">{params.id}</p>
        </div>
      </div>

      <div class="metrics-grid">
        <div class="metric-card power">
          <span class="metric-label">Power</span>
          <span class="metric-value">
            {formatPower(latestReading?.power)}
          </span>
        </div>
        <div class="metric-card voltage">
          <span class="metric-label">Voltage</span>
          <span class="metric-value">
            {formatVoltage(latestReading?.voltage)}
          </span>
        </div>
        <div class="metric-card current">
          <span class="metric-label">Current</span>
          <span class="metric-value">
            {formatCurrent(latestReading?.current)}
          </span>
        </div>
        <div class="metric-card energy">
          <span class="metric-label">Energy Consumed</span>
          <span class="metric-value">
            {formatEnergy(latestReading?.energy)}
          </span>
        </div>
      </div>

      <div class="secondary-metrics">
        <div class="secondary-metric">
          <span class="secondary-label">Produced Energy</span>
          <span class="secondary-value">
            {formatEnergy(latestReading?.produced_energy)}
          </span>
        </div>
        <div class="secondary-metric">
          <span class="secondary-label">AC Frequency</span>
          <span class="secondary-value">
            {formatFrequency(latestReading?.ac_frequency)}
          </span>
        </div>
        <div class="secondary-metric">
          <span class="secondary-label">Power Factor</span>
          <span class="secondary-value">
            {formatPowerFactor(latestReading?.power_factor)}
          </span>
        </div>
      </div>

      <div class="status-section">
        <h2 class="section-title">Device Status</h2>
        <div class="status-grid">
          {#if deviceState?.battery_level != null}
            <div class="status-item">
              <StatusBadge type="battery" value={deviceState.battery_level} />
            </div>
          {/if}
          {#if deviceState?.link_quality != null}
            <div class="status-item">
              <StatusBadge type="signal" value={deviceState.link_quality} />
            </div>
          {/if}
          <div class="status-item last-seen">
            <span class="status-label">Last seen</span>
            <span class="status-value">
              {formatLastSeen(
                deviceState?.last_seen || latestReading?.timestamp,
              )}
            </span>
          </div>
        </div>
      </div>

      <div class="chart-section">
        <h2 class="section-title">Power History</h2>

        <div class="toolbar">
          <div class="toolbar-section">
            <span class="section-label">Range:</span>
            <div class="button-group">
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
            {timeRange}
            metricType="power"
          />
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .energy-detail-page {
    min-height: calc(100vh - var(--app-header-height) - var(--app-nav-height));
    background: #f3f4f6;
    padding: 1.5rem;
  }

  .page-header {
    margin-bottom: 1.5rem;
  }

  .back-button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
    cursor: pointer;
    transition: all 0.15s;
  }

  .back-button:hover {
    background: #f9fafb;
    border-color: #9ca3af;
  }

  .loading-state,
  .error-state {
    text-align: center;
    padding: 4rem 2rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(59, 130, 246, 0.2);
    border-top-color: #3b82f6;
    border-radius: 50%;
    margin: 0 auto 1rem;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .content {
    max-width: 900px;
    margin: 0 auto;
  }

  .device-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    margin-bottom: 1.5rem;
  }

  .device-icon {
    width: 64px;
    height: 64px;
    background: linear-gradient(135deg, #fbbf24, #f59e0b);
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-emoji {
    font-size: 32px;
  }

  .device-info {
    flex: 1;
  }

  .device-name {
    font-size: 1.5rem;
    font-weight: 600;
    color: #111827;
    margin: 0 0 0.25rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .edit-name-btn {
    background: none;
    border: none;
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s;
    padding: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .edit-name-btn:hover {
    opacity: 1;
  }

  .edit-name-btn svg {
    width: 18px;
    height: 18px;
    color: #6b7280;
  }

  .device-id {
    font-size: 0.8125rem;
    color: #6b7280;
    font-family: monospace;
    margin: 0;
  }

  .name-edit {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .name-input {
    font-size: 1.25rem;
    font-weight: 600;
    padding: 0.375rem 0.75rem;
    border: 2px solid #3b82f6;
    border-radius: 6px;
    outline: none;
    min-width: 200px;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-sm {
    padding: 0.375rem 0.75rem;
    font-size: 0.8125rem;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover {
    background: #2563eb;
  }

  .btn-secondary {
    background: #e5e7eb;
    color: #374151;
  }

  .btn-secondary:hover {
    background: #d1d5db;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .metric-card {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .metric-card.power {
    border-left: 4px solid #f59e0b;
  }

  .metric-card.voltage {
    border-left: 4px solid #8b5cf6;
  }

  .metric-card.current {
    border-left: 4px solid #10b981;
  }

  .metric-card.energy {
    border-left: 4px solid #3b82f6;
  }

  .metric-label {
    font-size: 0.875rem;
    color: #6b7280;
    font-weight: 500;
  }

  .metric-value {
    font-size: 2rem;
    font-weight: 700;
    color: #111827;
    letter-spacing: -0.02em;
  }

  .secondary-metrics {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .secondary-metric {
    flex: 1;
    min-width: 120px;
    background: white;
    padding: 1rem;
    border-radius: 8px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .secondary-label {
    font-size: 0.75rem;
    color: #9ca3af;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .secondary-value {
    font-size: 1.125rem;
    font-weight: 600;
    color: #374151;
  }

  .status-section,
  .chart-section {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
    margin: 0 0 1rem;
  }

  .status-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: center;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-item.last-seen {
    padding: 0.5rem 0.75rem;
    background: #f3f4f6;
    border-radius: 8px;
  }

  .status-label {
    font-size: 0.8125rem;
    color: #6b7280;
  }

  .status-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: #f9fafb;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .toolbar-section {
    display: flex;
    align-items: center;
    gap: 0.625rem;
  }

  .section-label {
    font-size: 0.8125rem;
    color: #6b7280;
    font-weight: 500;
  }

  .button-group {
    display: flex;
    gap: 0.375rem;
    background: white;
    padding: 0.25rem;
    border-radius: 6px;
    border: 1px solid #d1d5db;
  }

  .toolbar-btn {
    padding: 0.375rem 0.75rem;
    background: transparent;
    border: none;
    border-radius: 4px;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .toolbar-btn:hover {
    background: #f3f4f6;
    color: #111827;
  }

  .toolbar-btn.active {
    background: #3b82f6;
    color: white;
  }

  .chart-container {
    height: 400px;
    border-radius: 8px;
    overflow: hidden;
  }

  @media (max-width: 640px) {
    .energy-detail-page {
      padding: 1rem;
    }

    .device-header {
      flex-direction: column;
      text-align: center;
    }

    .device-name {
      justify-content: center;
    }

    .metrics-grid {
      grid-template-columns: 1fr;
    }

    .metric-value {
      font-size: 1.75rem;
    }

    .secondary-metrics {
      flex-direction: column;
    }

    .secondary-metric {
      min-width: auto;
    }

    .toolbar {
      flex-direction: column;
      align-items: stretch;
      gap: 0.75rem;
    }

    .toolbar-section {
      justify-content: space-between;
    }

    .button-group {
      flex: 1;
    }

    .toolbar-btn {
      flex: 1;
      text-align: center;
    }

    .chart-container {
      height: 300px;
    }

    .name-edit {
      flex-wrap: wrap;
    }

    .name-input {
      flex: 1;
      min-width: 150px;
    }
  }
</style>
