<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    fetchLogView,
    type SystemMonitorEntry,
    type ProcessMonitorEntry,
    type TopConsumerEntry,
    type LogEntry,
  } from "./api";
  import SystemMetricsView from "./SystemMetricsView.svelte";
  import ProcessTableView from "./ProcessTableView.svelte";
  import TopConsumersView from "./TopConsumersView.svelte";
  import { cache } from "./stores/cache";

  type Tab = "system" | "processes" | "top-cpu" | "top-ram";
  type TimeRange = "24h" | "1w" | "1m" | "1y";

  let activeTab = $state<Tab>("system");
  let selectedTimeRange = $state<TimeRange>("24h");
  let systemEntries = $state<SystemMonitorEntry[]>([]);
  let processEntries = $state<ProcessMonitorEntry[]>([]);
  let topCpuEntries = $state<TopConsumerEntry[]>([]);
  let topRamEntries = $state<TopConsumerEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let pollInterval = $state<number | null>(null);

  // Map time ranges to approximate number of log lines
  // monitor.sh writes every 60 seconds, so 60 lines = 1 hour
  const timeRangeToLines: Record<TimeRange, number> = {
    "24h": 1440, // 24 hours
    "1w": 10080, // 7 days
    "1m": 43200, // 30 days
    "1y": 525600, // 365 days
  };

  // Time range in milliseconds for client-side filtering
  const timeRangeToMs: Record<TimeRange, number> = {
    "24h": 24 * 60 * 60 * 1000,
    "1w": 7 * 24 * 60 * 60 * 1000,
    "1m": 30 * 24 * 60 * 60 * 1000,
    "1y": 365 * 24 * 60 * 60 * 1000,
  };

  // Filter entries by time range
  function filterByTimeRange(entries: LogEntry[]): LogEntry[] {
    const cutoff = Date.now() - timeRangeToMs[selectedTimeRange];
    return entries.filter((e) => {
      const ts = "timestamp" in e ? e.timestamp : "";
      return new Date(ts).getTime() >= cutoff;
    });
  }

  async function loadLogFile(
    filename: string,
    filterFn: (e: any) => boolean
  ): Promise<LogEntry[]> {
    // Always fetch based on time range (line count approximation)
    const maxLines = timeRangeToLines[selectedTimeRange];
    const result = await fetchLogView(filename, maxLines);
    cache.setLogEntries(filename, result.entries, result.totalLines);

    // Apply type filter and time-based filter
    const typeFiltered = result.entries.filter(filterFn) as LogEntry[];
    return filterByTimeRange(typeFiltered);
  }

  async function loadSystemMetrics() {
    try {
      systemEntries = (await loadLogFile(
        "system_monitor.log",
        (e: any) => "cpu_usage" in e
      )) as SystemMonitorEntry[];
    } catch (e) {
      console.error("Failed to load system metrics:", e);
      error = "Failed to load system metrics";
    }
  }

  async function loadProcessMetrics() {
    try {
      processEntries = (await loadLogFile(
        "process_monitor.log",
        (e: any) => "process" in e && "status" in e
      )) as ProcessMonitorEntry[];
    } catch (e) {
      console.error("Failed to load process metrics:", e);
      error = "Failed to load process metrics";
    }
  }

  async function loadTopCpuConsumers() {
    try {
      topCpuEntries = (await loadLogFile(
        "top_cpu_consumers.log",
        (e: any) => "rank" in e
      )) as TopConsumerEntry[];
    } catch (e) {
      console.error("Failed to load top CPU consumers:", e);
      error = "Failed to load top CPU consumers";
    }
  }

  async function loadTopRamConsumers() {
    try {
      topRamEntries = (await loadLogFile(
        "top_ram_consumers.log",
        (e: any) => "rank" in e
      )) as TopConsumerEntry[];
    } catch (e) {
      console.error("Failed to load top RAM consumers:", e);
      error = "Failed to load top RAM consumers";
    }
  }

  async function loadAllData(showSpinner = false) {
    if (showSpinner) {
      loading = true;
    }
    error = null;

    await Promise.all([
      loadSystemMetrics(),
      loadProcessMetrics(),
      loadTopCpuConsumers(),
      loadTopRamConsumers(),
    ]);

    loading = false;
  }

  function setActiveTab(tab: Tab) {
    activeTab = tab;
  }

  async function setTimeRange(range: TimeRange) {
    if (range === selectedTimeRange) return;
    selectedTimeRange = range;
    // Clear cache and reload with new time range
    cache.clearAll();
    await loadAllData(true);
  }

  onMount(() => {
    loadAllData();

    // Poll every 15 seconds (using delta updates)
    pollInterval = window.setInterval(loadAllData, 15000);
  });

  onDestroy(() => {
    if (pollInterval !== null) {
      clearInterval(pollInterval);
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
        <div class="refresh-indicator">Auto-refresh: 15s</div>
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
        <button onclick={() => loadAllData(true)}>Retry</button>
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
    color: #6b7280;
    padding: 0.5rem 1rem;
    background-color: #f3f4f6;
    border-radius: 6px;
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
