<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    fetchLogsSince,
    type SystemMonitorEntry,
    type ProcessMonitorEntry,
    type TopConsumerEntry,
    type TimeRange,
  } from "../api";
  import SystemMetricsView from "./SystemMetricsView.svelte";
  import ProcessTableView from "./ProcessTableView.svelte";
  import TopConsumersView from "./TopConsumersView.svelte";
  import { cache } from "../stores/cache";
  import { eventStream, type LogEntriesEvent } from "../websocket";

  type Tab = "system" | "processes" | "top-cpu" | "top-ram";

  // Map tabs to log filenames
  const TAB_TO_FILE: Record<Tab, string> = {
    system: "system_monitor.log",
    processes: "process_monitor.log",
    "top-cpu": "top_cpu_consumers.log",
    "top-ram": "top_ram_consumers.log",
  };

  let activeTab = $state<Tab>("system");
  let selectedTimeRange = $state<TimeRange>("24h");
  let systemEntries = $state<SystemMonitorEntry[]>([]);
  let processEntries = $state<ProcessMonitorEntry[]>([]);
  let topCpuEntries = $state<TopConsumerEntry[]>([]);
  let topRamEntries = $state<TopConsumerEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Track which tabs have been loaded for the current time range
  let loadedTabs = $state<Set<Tab>>(new Set());

  // Time range in milliseconds for client-side filtering
  const timeRangeToMs: Record<TimeRange, number> = {
    "24h": 24 * 60 * 60 * 1000,
    "1w": 7 * 24 * 60 * 60 * 1000,
    "1m": 30 * 24 * 60 * 60 * 1000,
    "1y": 365 * 24 * 60 * 60 * 1000,
  };

  // Filter entries by time range (for when we have cached data from a wider range)
  function filterByTimeRange<T extends { timestamp: string }>(entries: T[]): T[] {
    const cutoff = Date.now() - timeRangeToMs[selectedTimeRange];
    return entries.filter((e) => new Date(e.timestamp).getTime() >= cutoff);
  }

  // Load data for a specific tab using the new efficient API
  async function loadTabData(tab: Tab): Promise<void> {
    const filename = TAB_TO_FILE[tab];

    // Check if already loaded for this time range
    if (cache.isLogLoaded(filename, selectedTimeRange)) {
      // Use cached data
      updateEntriesFromCache(tab);
      return;
    }

    try {
      const entries = await fetchLogsSince(filename, selectedTimeRange);
      cache.setLogEntries(filename, entries, selectedTimeRange);
      updateEntriesFromCache(tab);
    } catch (e) {
      console.error(`Failed to load ${tab}:`, e);
      throw e;
    }
  }

  // Update component state from cache
  function updateEntriesFromCache(tab: Tab): void {
    const filename = TAB_TO_FILE[tab];
    const entries = cache.getLogEntries(filename);

    switch (tab) {
      case "system":
        systemEntries = filterByTimeRange(
          entries.filter((e): e is SystemMonitorEntry => "cpu_usage" in e)
        );
        break;
      case "processes":
        processEntries = filterByTimeRange(
          entries.filter((e): e is ProcessMonitorEntry => "status" in e)
        );
        break;
      case "top-cpu":
        topCpuEntries = filterByTimeRange(
          entries.filter((e): e is TopConsumerEntry => "rank" in e)
        );
        break;
      case "top-ram":
        topRamEntries = filterByTimeRange(
          entries.filter((e): e is TopConsumerEntry => "rank" in e)
        );
        break;
    }
  }

  // Load data for the active tab only (lazy loading)
  async function loadActiveTab(showSpinner = false): Promise<void> {
    if (showSpinner) {
      loading = true;
    }
    error = null;

    try {
      await loadTabData(activeTab);
      loadedTabs.add(activeTab);
      loadedTabs = loadedTabs; // Trigger reactivity
    } catch (e) {
      console.error("Failed to load tab data:", e);
      error = `Failed to load ${activeTab} data`;
    } finally {
      loading = false;
    }
  }

  function setActiveTab(tab: Tab): void {
    if (tab === activeTab) return;
    activeTab = tab;

    // Load data for new tab if not already loaded
    if (!loadedTabs.has(tab)) {
      loadActiveTab(true);
    } else {
      // Refresh from cache (applies current time range filter)
      updateEntriesFromCache(tab);
    }
  }

  async function setTimeRange(range: TimeRange): Promise<void> {
    if (range === selectedTimeRange) return;
    selectedTimeRange = range;

    // Clear loaded tabs tracking - need to reload for new time range
    loadedTabs.clear();
    loadedTabs = loadedTabs;

    // Clear cache and reload current tab
    cache.clearAllLogs();
    await loadActiveTab(true);
  }

  // Handle WebSocket log events
  function handleLogEvent(event: LogEntriesEvent): void {
    // Append new entries to cache
    cache.appendLogEntries(event.log_file, event.entries);

    // Update UI if this is the active tab
    const activeFilename = TAB_TO_FILE[activeTab];
    const eventFilenames: Record<string, string> = {
      system_monitor: "system_monitor.log",
      process_monitor: "process_monitor.log",
      top_cpu_consumers: "top_cpu_consumers.log",
      top_ram_consumers: "top_ram_consumers.log",
    };

    if (eventFilenames[event.log_file] === activeFilename) {
      updateEntriesFromCache(activeTab);
    }
  }

  // WebSocket subscription
  let unsubscribe: (() => void) | null = null;

  onMount(() => {
    // Initial load of active tab
    loadActiveTab(true);

    // Connect to WebSocket for real-time updates
    eventStream.connect();

    // Subscribe to events
    unsubscribe = eventStream.events.subscribe((event) => {
      if (event?.event === "log_entries") {
        handleLogEvent(event as LogEntriesEvent);
      }
    });
  });

  onDestroy(() => {
    if (unsubscribe) {
      unsubscribe();
    }
  });
</script>

<div class="logs-page">
  <div class="header">
    <h2>System Logs</h2>
    <div class="header-controls">
      <div class="time-range-selector">
        <span class="time-range-label">Time range:</span>
        {#each ["24h", "1w", "1m", "1y"] as range (range)}
          <button
            class="time-range-btn"
            class:active={selectedTimeRange === range}
            onclick={() => setTimeRange(range as TimeRange)}
          >
            {range}
          </button>
        {/each}
      </div>
      {#if !loading && !error}
        <div class="refresh-indicator">Live updates via WebSocket</div>
      {/if}
    </div>
  </div>

  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === "system"}
      onclick={() => setActiveTab("system")}
    >
      System Metrics
    </button>
    <button
      class="tab"
      class:active={activeTab === "processes"}
      onclick={() => setActiveTab("processes")}
    >
      Processes
    </button>
    <button
      class="tab"
      class:active={activeTab === "top-cpu"}
      onclick={() => setActiveTab("top-cpu")}
    >
      Top CPU
    </button>
    <button
      class="tab"
      class:active={activeTab === "top-ram"}
      onclick={() => setActiveTab("top-ram")}
    >
      Top RAM
    </button>
  </div>

  <div class="content">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading log data...</p>
      </div>
    {:else if error}
      <div class="error-state">
        <p>{error}</p>
        <button onclick={() => loadActiveTab(true)}>Retry</button>
      </div>
    {:else if activeTab === "system"}
      <SystemMetricsView entries={systemEntries} />
    {:else if activeTab === "processes"}
      <ProcessTableView
        entries={processEntries}
        timeRange={selectedTimeRange}
      />
    {:else if activeTab === "top-cpu"}
      <TopConsumersView entries={topCpuEntries} type="cpu" />
    {:else if activeTab === "top-ram"}
      <TopConsumersView entries={topRamEntries} type="ram" />
    {/if}
  </div>
</div>

<style>
  .logs-page {
    width: 100%;
    max-width: 1400px;
    margin: 0 auto;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #111827;
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }

  .time-range-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background-color: #f9fafb;
    padding: 0.25rem;
    border-radius: 6px;
    border: 1px solid #e5e7eb;
  }

  .time-range-label {
    font-size: 0.8125rem;
    color: #6b7280;
    font-weight: 500;
    padding: 0 0.5rem;
  }

  .time-range-btn {
    padding: 0.375rem 0.75rem;
    background: transparent;
    border: none;
    border-radius: 4px;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.15s;
  }

  .time-range-btn:hover {
    background: #e5e7eb;
    color: #111827;
  }

  .time-range-btn.active {
    background: #3b82f6;
    color: white;
  }

  .refresh-indicator {
    font-size: 0.875rem;
    color: #059669;
    padding: 0.5rem 1rem;
    background-color: #ecfdf5;
    border-radius: 6px;
    border: 1px solid #d1fae5;
  }

  .tabs {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1.5rem;
    border-bottom: 2px solid #e5e7eb;
    overflow-x: auto;
  }

  .tab {
    padding: 0.75rem 1.5rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .tab:hover {
    color: #111827;
    background-color: #f9fafb;
  }

  .tab.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  .content {
    min-height: 400px;
  }

  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
  }

  .loading-state p,
  .error-state p {
    margin: 1rem 0;
    color: #6b7280;
    font-size: 1rem;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-state button {
    margin-top: 1rem;
    padding: 0.5rem 1.5rem;
    background-color: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .error-state button:hover {
    background-color: #2563eb;
  }

  @media (max-width: 768px) {
    .header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .header-controls {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.75rem;
      width: 100%;
    }

    .time-range-selector {
      width: 100%;
      justify-content: space-between;
    }

    .time-range-btn {
      flex: 1;
    }

    .tabs {
      gap: 0;
    }

    .tab {
      padding: 0.75rem 1rem;
      font-size: 0.8125rem;
    }
  }
</style>
