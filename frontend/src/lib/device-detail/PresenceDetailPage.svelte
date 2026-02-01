<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { deviceStateMemory, sensorsMemory } from "../memory";
  import type { PresenceSensorReading } from "../api/sensors";
  import type { DeviceInfo, DeviceState } from "../types/devices";
  import { TIME_RANGE_HOURS } from "../stores/graphConfig";
  import StatusBadge from "../shared/StatusBadge.svelte";
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

  // Time range state for history
  type TimeRange = "24h" | "1w" | "1m" | "1y";
  let timeRange = $state<TimeRange>("24h");

  // Presence history
  let presenceHistory = $state<PresenceSensorReading[]>([]);
  let loadingHistory = $state(false);

  // Get the latest reading for this presence sensor
  const latestReading = $derived.by(() => {
    const reading = $sensorsMemory.latestByDevice[params.id] ?? null;
    if (!reading || reading.type !== "presence") return null;
    return reading as PresenceSensorReading;
  });

  const deviceName = $derived.by(() => {
    const memoryDevice = $sensorsMemory.devices.find(
      (d) => d.device_id === params.id,
    );
    return memoryDevice?.name ?? device?.name ?? params.id;
  });

  // Detect presence state changes in history for timeline
  const presenceEvents = $derived.by(() => {
    if (presenceHistory.length === 0) return [];

    const events: Array<{
      timestamp: Date;
      occupied: boolean;
      duration?: number;
    }> = [];

    let lastState: boolean | null = null;
    let lastTimestamp: Date | null = null;

    for (const reading of presenceHistory) {
      if (lastState === null || reading.occupied !== lastState) {
        if (events.length > 0 && lastTimestamp) {
          events[events.length - 1].duration =
            reading.timestamp.getTime() - lastTimestamp.getTime();
        }
        events.push({
          timestamp: reading.timestamp,
          occupied: reading.occupied,
        });
        lastState = reading.occupied;
        lastTimestamp = reading.timestamp;
      }
    }

    if (events.length > 0 && lastTimestamp) {
      events[events.length - 1].duration = Date.now() - lastTimestamp.getTime();
    }

    return events.reverse();
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
        error = "Presence sensor not found";
        loading = false;
        return;
      }

      device = deviceInfo;

      const state = await deviceStateMemory.ensureDeviceState(params.id);
      if (state) {
        deviceState = state;
      }

      await loadHistoryForRange(timeRange);
    } catch (err) {
      console.error("Failed to load presence sensor data:", err);
      error =
        err instanceof Error
          ? err.message
          : "Failed to load presence sensor data";
    } finally {
      loading = false;
    }
  }

  async function loadHistoryForRange(range: TimeRange) {
    loadingHistory = true;
    const hours = TIME_RANGE_HOURS[range];
    try {
      await sensorsMemory.ensureRecentReadings(params.id, hours);
      const endSec = Math.floor(Date.now() / 1000);
      const startSec = endSec - hours * 3600;
      const readings = sensorsMemory.getReadings(
        params.id,
        startSec,
        endSec,
        0,
      );
      presenceHistory = readings.filter(
        (r): r is PresenceSensorReading => r.type === "presence",
      );
    } catch (err) {
      console.error("Failed to load presence history:", err);
    } finally {
      loadingHistory = false;
    }
  }

  function handleTimeRangeChange(range: TimeRange) {
    timeRange = range;
    void loadHistoryForRange(range);
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

  function formatDuration(ms: number | undefined): string {
    if (!ms) return "";
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ${hours % 24}h`;
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m`;
    return `${seconds}s`;
  }

  function formatTimestamp(date: Date): string {
    const now = new Date();
    const isToday = date.toDateString() === now.toDateString();
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);
    const isYesterday = date.toDateString() === yesterday.toDateString();

    const timeStr = date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });

    if (isToday) return `Today ${timeStr}`;
    if (isYesterday) return `Yesterday ${timeStr}`;
    return `${date.toLocaleDateString([], { month: "short", day: "numeric" })} ${timeStr}`;
  }

  // Stats calculations
  const stats = $derived.by(() => {
    if (presenceEvents.length === 0) return null;

    let totalOccupiedTime = 0;
    let totalUnoccupiedTime = 0;
    let occupiedCount = 0;

    for (const event of presenceEvents) {
      if (event.duration) {
        if (event.occupied) {
          totalOccupiedTime += event.duration;
          occupiedCount++;
        } else {
          totalUnoccupiedTime += event.duration;
        }
      }
    }

    const totalTime = totalOccupiedTime + totalUnoccupiedTime;
    const occupancyRate =
      totalTime > 0 ? (totalOccupiedTime / totalTime) * 100 : 0;

    return {
      occupiedCount,
      totalOccupiedTime,
      occupancyRate,
    };
  });
</script>

<div class="detail-page">
  <!-- Radar sweep effect background -->
  <div class="ambient-bg"></div>
  <div class="radar-sweep" class:active={latestReading?.occupied}></div>

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
      <p>Loading presence sensor data...</p>
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
            <div class="device-icon" class:active={latestReading?.occupied}>
              <svg viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"
                />
              </svg>
            </div>
            {#if latestReading?.occupied}
              <div class="detection-ring"></div>
              <div class="detection-ring delay-1"></div>
              <div class="detection-ring delay-2"></div>
            {/if}
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

        <!-- Presence Status Display -->
        <div class="presence-display" class:detected={latestReading?.occupied}>
          <div class="presence-visual">
            <div class="presence-circle">
              <div class="presence-inner">
                {#if latestReading?.occupied}
                  <svg
                    viewBox="0 0 24 24"
                    fill="currentColor"
                    class="presence-icon"
                  >
                    <path
                      d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"
                    />
                  </svg>
                {:else}
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    class="presence-icon"
                  >
                    <circle cx="12" cy="12" r="10" />
                    <path d="M8 12h8" />
                  </svg>
                {/if}
              </div>
            </div>
            {#if latestReading?.occupied}
              <div class="ripple"></div>
              <div class="ripple delay-1"></div>
            {/if}
          </div>

          <div class="presence-info">
            <span class="presence-status">
              {latestReading?.occupied ? "Motion Detected" : "No Motion"}
            </span>
            {#if latestReading?.illumination !== undefined}
              <div class="illumination-badge">
                <svg
                  viewBox="0 0 24 24"
                  fill="currentColor"
                  width="14"
                  height="14"
                >
                  <path
                    d="M12 7c-2.76 0-5 2.24-5 5s2.24 5 5 5 5-2.24 5-5-2.24-5-5-5zM2 13h2c.55 0 1-.45 1-1s-.45-1-1-1H2c-.55 0-1 .45-1 1s.45 1 1 1zm18 0h2c.55 0 1-.45 1-1s-.45-1-1-1h-2c-.55 0-1 .45-1 1s.45 1 1 1zM11 2v2c0 .55.45 1 1 1s1-.45 1-1V2c0-.55-.45-1-1-1s-1 .45-1 1zm0 18v2c0 .55.45 1 1 1s1-.45 1-1v-2c0-.55-.45-1-1-1s-1 .45-1 1z"
                  />
                </svg>
                <span>{latestReading.illumination} lux</span>
              </div>
            {/if}
          </div>
        </div>
      </section>

      <!-- Stats Grid -->
      {#if stats}
        <section class="stats-section">
          <div class="stat-card">
            <div class="stat-icon">
              <svg viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z"
                />
              </svg>
            </div>
            <div class="stat-content">
              <span class="stat-value">{stats.occupiedCount}</span>
              <span class="stat-label">Detections</span>
            </div>
          </div>

          <div class="stat-card">
            <div class="stat-icon time">
              <svg viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z"
                />
              </svg>
            </div>
            <div class="stat-content">
              <span class="stat-value"
                >{formatDuration(stats.totalOccupiedTime)}</span
              >
              <span class="stat-label">Occupied Time</span>
            </div>
          </div>

          <div class="stat-card highlight">
            <div class="stat-icon rate">
              <svg viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-5 14H7v-2h7v2zm3-4H7v-2h10v2zm0-4H7V7h10v2z"
                />
              </svg>
            </div>
            <div class="stat-content">
              <span class="stat-value">{stats.occupancyRate.toFixed(0)}%</span>
              <span class="stat-label">Occupancy Rate</span>
            </div>
            <div class="stat-bar">
              <div
                class="stat-bar-fill"
                style="width: {stats.occupancyRate}%"
              ></div>
            </div>
          </div>
        </section>
      {/if}

      <!-- History Section -->
      <section class="history-section">
        <div class="section-header">
          <h2>Detection Timeline</h2>
          <div class="toolbar">
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

        {#if loadingHistory}
          <div class="loading-history">
            <div class="mini-spinner"></div>
            <span>Loading timeline...</span>
          </div>
        {:else if presenceEvents.length === 0}
          <div class="empty-state">
            <svg viewBox="0 0 24 24" fill="currentColor" class="empty-icon">
              <path
                d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"
              />
            </svg>
            <p>No presence events in selected period</p>
          </div>
        {:else}
          <div class="timeline">
            {#each presenceEvents as event, index (event.timestamp.getTime())}
              <div class="timeline-item" class:occupied={event.occupied}>
                <div class="timeline-connector">
                  <div class="connector-dot"></div>
                  {#if index < presenceEvents.length - 1}
                    <div class="connector-line"></div>
                  {/if}
                </div>
                <div class="timeline-card">
                  <div class="event-status">
                    <span class="status-badge" class:active={event.occupied}>
                      {event.occupied ? "Detected" : "Clear"}
                    </span>
                    {#if event.duration}
                      <span class="event-duration"
                        >{formatDuration(event.duration)}</span
                      >
                    {/if}
                  </div>
                  <span class="event-time"
                    >{formatTimestamp(event.timestamp)}</span
                  >
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>
  {/if}
</div>

<style>
  /* === Design Tokens === */
  .detail-page {
    --accent-primary: #8b5cf6;
    --accent-detected: #f59e0b;
    --accent-clear: #64748b;
    --accent-danger: #ef4444;
    --surface-elevated: rgba(255, 255, 255, 0.98);
    --text-primary: #0f172a;
    --text-secondary: #475569;
    --text-muted: #94a3b8;
    --border-subtle: rgba(0, 0, 0, 0.06);
  }

  .detail-page {
    position: relative;
    min-height: 100vh;
    background: linear-gradient(180deg, #faf5ff 0%, #f5f3ff 50%, #ede9fe 100%);
    padding: var(--space-4);
    overflow-x: hidden;
  }

  /* Ambient backgrounds */
  .ambient-bg {
    position: fixed;
    inset: 0;
    background: radial-gradient(
        ellipse at 30% 20%,
        rgba(139, 92, 246, 0.06) 0%,
        transparent 50%
      ),
      radial-gradient(
        ellipse at 70% 80%,
        rgba(245, 158, 11, 0.04) 0%,
        transparent 50%
      );
    pointer-events: none;
    z-index: 0;
  }

  .radar-sweep {
    position: fixed;
    top: 50%;
    left: 50%;
    width: 200vw;
    height: 200vw;
    transform: translate(-50%, -50%);
    background: conic-gradient(
      from 0deg,
      transparent 0deg,
      transparent 355deg,
      rgba(139, 92, 246, 0) 360deg
    );
    pointer-events: none;
    z-index: 0;
    opacity: 0;
    transition: opacity 0.5s ease;
  }

  .radar-sweep.active {
    opacity: 1;
    background: conic-gradient(
      from 0deg,
      transparent 0deg,
      rgba(245, 158, 11, 0.1) 20deg,
      transparent 90deg
    );
    animation: radar-rotate 4s linear infinite;
  }

  @keyframes radar-rotate {
    from {
      transform: translate(-50%, -50%) rotate(0deg);
    }
    to {
      transform: translate(-50%, -50%) rotate(360deg);
    }
  }

  /* === Header === */
  .page-header {
    position: relative;
    z-index: 10;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-6);
    max-width: 800px;
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

  /* === Loading / Error States === */
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
    border-top-color: var(--accent-detected);
    animation-delay: 0.15s;
  }

  .loader-ring:nth-child(3) {
    inset: 16px;
    border-top-color: #10b981;
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
    max-width: 800px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
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
    background: linear-gradient(135deg, #a78bfa, #8b5cf6);
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    transition: all 0.4s ease;
  }

  .device-icon.active {
    background: linear-gradient(135deg, #fbbf24, #f59e0b);
    box-shadow: 0 4px 20px rgba(245, 158, 11, 0.4);
  }

  .device-icon svg {
    width: 28px;
    height: 28px;
  }

  .detection-ring {
    position: absolute;
    inset: -8px;
    border: 2px solid rgba(245, 158, 11, 0.4);
    border-radius: 24px;
    animation: detection-pulse 1.5s ease-out infinite;
  }

  .detection-ring.delay-1 {
    animation-delay: 0.5s;
  }
  .detection-ring.delay-2 {
    animation-delay: 1s;
  }

  @keyframes detection-pulse {
    0% {
      transform: scale(1);
      opacity: 1;
    }
    100% {
      transform: scale(1.4);
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

  /* === Presence Display === */
  .presence-display {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-6) var(--space-4);
    background: linear-gradient(135deg, #f8fafc, #f1f5f9);
    border-radius: var(--radius-lg);
    transition: all 0.4s ease;
  }

  .presence-display.detected {
    background: linear-gradient(135deg, #fffbeb, #fef3c7);
  }

  .presence-visual {
    position: relative;
  }

  .presence-circle {
    width: 100px;
    height: 100px;
    border-radius: 50%;
    background: white;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
    transition: all 0.4s ease;
  }

  .presence-display.detected .presence-circle {
    box-shadow: 0 4px 30px rgba(245, 158, 11, 0.25);
  }

  .presence-inner {
    width: 70px;
    height: 70px;
    border-radius: 50%;
    background: #f1f5f9;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.4s ease;
  }

  .presence-display.detected .presence-inner {
    background: linear-gradient(135deg, #fbbf24, #f59e0b);
  }

  .presence-icon {
    width: 32px;
    height: 32px;
    color: #94a3b8;
    transition: color 0.4s ease;
  }

  .presence-display.detected .presence-icon {
    color: white;
  }

  .ripple {
    position: absolute;
    inset: -10px;
    border: 2px solid rgba(245, 158, 11, 0.4);
    border-radius: 50%;
    animation: ripple-expand 2s ease-out infinite;
  }

  .ripple.delay-1 {
    animation-delay: 1s;
  }

  @keyframes ripple-expand {
    0% {
      transform: scale(1);
      opacity: 0.6;
    }
    100% {
      transform: scale(1.6);
      opacity: 0;
    }
  }

  .presence-info {
    text-align: center;
  }

  .presence-status {
    display: block;
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: var(--space-2);
  }

  .illumination-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-3);
    background: white;
    border-radius: var(--radius-pill);
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  /* === Stats Section === */
  .stats-section {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-3);
  }

  .stat-card {
    background: var(--surface-elevated);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
    border: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .stat-card.highlight {
    background: linear-gradient(135deg, #faf5ff, #f5f3ff);
    border-color: rgba(139, 92, 246, 0.2);
  }

  .stat-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: #f1f5f9;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .stat-icon svg {
    width: 20px;
    height: 20px;
  }

  .stat-icon.time {
    background: #ecfdf5;
    color: #10b981;
  }
  .stat-icon.rate {
    background: #f5f3ff;
    color: #8b5cf6;
  }

  .stat-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .stat-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-bar {
    height: 4px;
    background: #e2e8f0;
    border-radius: 2px;
    overflow: hidden;
  }

  .stat-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #8b5cf6, #a78bfa);
    border-radius: 2px;
    transition: width 0.6s ease;
  }

  /* === History Section === */
  .history-section {
    background: var(--surface-elevated);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
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
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
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

  .loading-history {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-8);
    color: var(--text-muted);
  }

  .mini-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid #e2e8f0;
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .empty-state {
    text-align: center;
    padding: var(--space-8);
    color: var(--text-muted);
  }

  .empty-icon {
    width: 48px;
    height: 48px;
    margin-bottom: var(--space-3);
    opacity: 0.5;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.9375rem;
  }

  /* === Timeline === */
  .timeline {
    display: flex;
    flex-direction: column;
  }

  .timeline-item {
    display: flex;
    gap: var(--space-3);
  }

  .timeline-connector {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 20px;
    flex-shrink: 0;
  }

  .connector-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #cbd5e1;
    border: 2px solid white;
    box-shadow: 0 0 0 2px #e2e8f0;
    flex-shrink: 0;
    z-index: 1;
  }

  .timeline-item.occupied .connector-dot {
    background: #f59e0b;
    box-shadow: 0 0 0 2px #fde68a;
  }

  .connector-line {
    flex: 1;
    width: 2px;
    background: #e2e8f0;
    min-height: 20px;
  }

  .timeline-card {
    flex: 1;
    padding: var(--space-3);
    background: #f8fafc;
    border-radius: var(--radius-md);
    margin-bottom: var(--space-3);
    transition: all 0.2s ease;
  }

  .timeline-item.occupied .timeline-card {
    background: #fffbeb;
  }

  .event-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
  }

  .status-badge {
    font-size: 0.8125rem;
    font-weight: 700;
    color: var(--text-muted);
  }

  .status-badge.active {
    color: #b45309;
  }

  .event-duration {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    background: white;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
  }

  .event-time {
    font-size: 0.75rem;
    color: var(--text-muted);
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
    background: #7c3aed;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);
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

    .stats-section {
      grid-template-columns: 1fr;
    }

    .stat-card {
      flex-direction: row;
      align-items: center;
    }

    .stat-card.highlight {
      flex-direction: column;
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
    }
  }
</style>
