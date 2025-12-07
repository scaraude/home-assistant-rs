<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    fetchLogView,
    type SystemMonitorEntry,
    type ProcessMonitorEntry,
    type TopConsumerEntry,
  } from './api';
  import SystemMetricsView from './SystemMetricsView.svelte';
  import ProcessTableView from './ProcessTableView.svelte';
  import TopConsumersView from './TopConsumersView.svelte';

  type Tab = 'system' | 'processes' | 'top-cpu' | 'top-ram';

  let activeTab: Tab = 'system';
  let systemEntries: SystemMonitorEntry[] = [];
  let processEntries: ProcessMonitorEntry[] = [];
  let topCpuEntries: TopConsumerEntry[] = [];
  let topRamEntries: TopConsumerEntry[] = [];
  let loading = true;
  let error: string | null = null;
  let pollInterval: number | null = null;

  async function loadSystemMetrics() {
    try {
      const entries = (await fetchLogView('system_monitor.log', 1000)) as any[];
      systemEntries = entries.filter((e) => 'cpu_usage' in e) as SystemMonitorEntry[];
    } catch (e) {
      console.error('Failed to load system metrics:', e);
      error = 'Failed to load system metrics';
    }
  }

  async function loadProcessMetrics() {
    try {
      const entries = (await fetchLogView('process_monitor.log', 1000)) as any[];
      processEntries = entries.filter((e) => 'process' in e && 'status' in e) as ProcessMonitorEntry[];
    } catch (e) {
      console.error('Failed to load process metrics:', e);
      error = 'Failed to load process metrics';
    }
  }

  async function loadTopCpuConsumers() {
    try {
      const entries = (await fetchLogView('top_cpu_consumers.log', 1000)) as any[];
      topCpuEntries = entries.filter((e) => 'rank' in e) as TopConsumerEntry[];
    } catch (e) {
      console.error('Failed to load top CPU consumers:', e);
      error = 'Failed to load top CPU consumers';
    }
  }

  async function loadTopRamConsumers() {
    try {
      const entries = (await fetchLogView('top_ram_consumers.log', 1000)) as any[];
      topRamEntries = entries.filter((e) => 'rank' in e) as TopConsumerEntry[];
    } catch (e) {
      console.error('Failed to load top RAM consumers:', e);
      error = 'Failed to load top RAM consumers';
    }
  }

  async function loadAllData() {
    loading = true;
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

  onMount(() => {
    loadAllData();

    // Poll every 15 seconds
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
    {#if !loading && !error}
      <div class="refresh-indicator">Auto-refresh: 15s</div>
    {/if}
  </div>

  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === 'system'}
      on:click={() => setActiveTab('system')}
    >
      System Metrics
    </button>
    <button
      class="tab"
      class:active={activeTab === 'processes'}
      on:click={() => setActiveTab('processes')}
    >
      Processes
    </button>
    <button
      class="tab"
      class:active={activeTab === 'top-cpu'}
      on:click={() => setActiveTab('top-cpu')}
    >
      Top CPU
    </button>
    <button
      class="tab"
      class:active={activeTab === 'top-ram'}
      on:click={() => setActiveTab('top-ram')}
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
        <button on:click={loadAllData}>Retry</button>
      </div>
    {:else}
      {#if activeTab === 'system'}
        <SystemMetricsView entries={systemEntries} />
      {:else if activeTab === 'processes'}
        <ProcessTableView entries={processEntries} />
      {:else if activeTab === 'top-cpu'}
        <TopConsumersView entries={topCpuEntries} type="cpu" />
      {:else if activeTab === 'top-ram'}
        <TopConsumersView entries={topRamEntries} type="ram" />
      {/if}
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
      gap: 0.5rem;
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
