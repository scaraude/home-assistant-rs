<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { devicesMemory, deviceStateMemory, sensorsMemory } from "../memory";
  import type { PresenceSensorReading } from "../api/sensors";
  import type { DeviceInfo, DeviceState } from "../types/devices";
  import { TIME_RANGE_HOURS } from "../stores/graphConfig";
  import StatusBadge from "../shared/StatusBadge.svelte";

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
        // State changed
        if (events.length > 0 && lastTimestamp) {
          // Calculate duration of previous state
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

    // Calculate duration for last event (until now)
    if (events.length > 0 && lastTimestamp) {
      events[events.length - 1].duration =
        Date.now() - lastTimestamp.getTime();
    }

    return events.reverse(); // Most recent first
  });

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = null;

    try {
      await sensorsMemory.ensureDevices();
      // Find device info from sensors memory
      const deviceInfo = $sensorsMemory.devices.find(
        (d) => d.device_id === params.id,
      );

      if (!deviceInfo) {
        error = "Presence sensor not found";
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

      // Load initial history
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
      const readings = sensorsMemory.getReadings(params.id, startSec, endSec, 0);
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

<div class="presence-detail-page">
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
      <p>Loading presence sensor data...</p>
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
        <div class="device-icon" class:active={latestReading?.occupied}>
          <span class="icon-emoji">{latestReading?.occupied ? "🟡" : "⚪"}</span>
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

      <div class="state-section">
        <div class="state-card">
          <div class="state-header">
            <span class="state-label">Current State</span>
          </div>
          <div class="presence-state" class:occupied={latestReading?.occupied}>
            <div class="presence-indicator">
              <div class="presence-dot"></div>
            </div>
            <span class="presence-text">
              {latestReading?.occupied ? "Presence Detected" : "No Presence"}
            </span>
          </div>
          {#if latestReading?.illumination}
            <div class="illumination">
              <span class="illumination-label">Illumination</span>
              <span class="illumination-value">{latestReading.illumination}</span>
            </div>
          {/if}
        </div>
      </div>

      {#if stats}
        <div class="stats-grid">
          <div class="stat-card">
            <span class="stat-label">Detections</span>
            <span class="stat-value">{stats.occupiedCount}</span>
            <span class="stat-sublabel">in selected period</span>
          </div>
          <div class="stat-card">
            <span class="stat-label">Total Occupied</span>
            <span class="stat-value">{formatDuration(stats.totalOccupiedTime)}</span>
            <span class="stat-sublabel">presence time</span>
          </div>
          <div class="stat-card">
            <span class="stat-label">Occupancy Rate</span>
            <span class="stat-value">{stats.occupancyRate.toFixed(1)}%</span>
            <span class="stat-sublabel">of time occupied</span>
          </div>
        </div>
      {/if}

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

      <div class="history-section">
        <h2 class="section-title">Detection History</h2>

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

        {#if loadingHistory}
          <div class="loading-history">
            <div class="spinner-small"></div>
            <span>Loading history...</span>
          </div>
        {:else if presenceEvents.length === 0}
          <div class="empty-history">
            <p>No presence events in selected period</p>
          </div>
        {:else}
          <div class="timeline">
            {#each presenceEvents as event (event.timestamp.getTime())}
              <div class="timeline-item" class:occupied={event.occupied}>
                <div class="timeline-marker">
                  <div class="marker-dot"></div>
                  <div class="marker-line"></div>
                </div>
                <div class="timeline-content">
                  <div class="event-header">
                    <span class="event-status">
                      {event.occupied ? "Presence detected" : "No presence"}
                    </span>
                    {#if event.duration}
                      <span class="event-duration">
                        {formatDuration(event.duration)}
                      </span>
                    {/if}
                  </div>
                  <span class="event-time">{formatTimestamp(event.timestamp)}</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .presence-detail-page {
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

  .spinner-small {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(59, 130, 246, 0.2);
    border-top-color: #3b82f6;
    border-radius: 50%;
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
    background: linear-gradient(135deg, #e5e7eb, #d1d5db);
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.3s ease;
  }

  .device-icon.active {
    background: linear-gradient(135deg, #fef3c7, #fde68a);
    box-shadow: 0 4px 12px rgba(251, 191, 36, 0.3);
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

  .state-section {
    margin-bottom: 1.5rem;
  }

  .state-card {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .state-header {
    margin-bottom: 1rem;
  }

  .state-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
  }

  .presence-state {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    background: #f9fafb;
    border-radius: 10px;
    border: 2px solid #e5e7eb;
    transition: all 0.3s ease;
  }

  .presence-state.occupied {
    background: #fefce8;
    border-color: #fde047;
  }

  .presence-indicator {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .presence-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: #d1d5db;
    transition: all 0.3s ease;
  }

  .presence-state.occupied .presence-dot {
    background: #eab308;
    box-shadow: 0 0 16px rgba(234, 179, 8, 0.6);
    animation: pulse-glow 2s ease-in-out infinite;
  }

  @keyframes pulse-glow {
    0%, 100% {
      box-shadow: 0 0 8px rgba(234, 179, 8, 0.4);
    }
    50% {
      box-shadow: 0 0 20px rgba(234, 179, 8, 0.8);
    }
  }

  .presence-text {
    font-size: 1.25rem;
    font-weight: 600;
    color: #374151;
  }

  .presence-state.occupied .presence-text {
    color: #854d0e;
  }

  .illumination {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #e5e7eb;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .illumination-label {
    font-size: 0.875rem;
    color: #6b7280;
  }

  .illumination-value {
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .stat-card {
    background: white;
    padding: 1.25rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    text-align: center;
  }

  .stat-label {
    font-size: 0.75rem;
    color: #9ca3af;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-value {
    font-size: 1.75rem;
    font-weight: 700;
    color: #111827;
  }

  .stat-sublabel {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .status-section,
  .history-section {
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

  .status-item .status-label {
    font-size: 0.8125rem;
    color: #6b7280;
  }

  .status-item .status-value {
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

  .loading-history {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 2rem;
    color: #6b7280;
  }

  .empty-history {
    text-align: center;
    padding: 2rem;
    color: #9ca3af;
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .timeline-item {
    display: flex;
    gap: 1rem;
    position: relative;
  }

  .timeline-marker {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 24px;
    flex-shrink: 0;
  }

  .marker-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #d1d5db;
    border: 2px solid white;
    box-shadow: 0 0 0 2px #d1d5db;
    z-index: 1;
  }

  .timeline-item.occupied .marker-dot {
    background: #eab308;
    box-shadow: 0 0 0 2px #fde047;
  }

  .marker-line {
    flex: 1;
    width: 2px;
    background: #e5e7eb;
    min-height: 24px;
  }

  .timeline-item:last-child .marker-line {
    display: none;
  }

  .timeline-content {
    flex: 1;
    padding-bottom: 1.25rem;
  }

  .event-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.25rem;
  }

  .event-status {
    font-size: 0.9375rem;
    font-weight: 600;
    color: #374151;
  }

  .timeline-item.occupied .event-status {
    color: #854d0e;
  }

  .event-duration {
    font-size: 0.8125rem;
    font-weight: 500;
    color: #6b7280;
    background: #f3f4f6;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
  }

  .event-time {
    font-size: 0.8125rem;
    color: #9ca3af;
  }

  @media (max-width: 640px) {
    .presence-detail-page {
      padding: 1rem;
    }

    .device-header {
      flex-direction: column;
      text-align: center;
    }

    .device-name {
      justify-content: center;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }

    .stat-value {
      font-size: 1.5rem;
    }

    .toolbar {
      flex-direction: column;
      align-items: stretch;
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

    .name-edit {
      flex-wrap: wrap;
      justify-content: center;
    }

    .name-input {
      flex: 1;
      min-width: 150px;
    }

    .event-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.25rem;
    }
  }
</style>
