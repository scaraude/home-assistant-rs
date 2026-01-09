<script lang="ts">
  import { onMount } from "svelte";
  import { networkTopologyStore } from "../stores/networkTopology";
  import { fetchNetworkTopology, refreshNetworkMap } from "../api";
  import NetworkGraph from "./NetworkGraph.svelte";

  let deviceStates: Map<
    string,
    { link_quality: number | null; battery_level: number | null }
  > = new Map();
  let refreshing = $state(false);
  let showRefreshWarning = $state(false);
  let isRefreshing = $derived(refreshing || $networkTopologyStore.loading);

  onMount(async () => {
    await loadTopology();
  });

  async function loadTopology() {
    networkTopologyStore.setLoading(true);
    try {
      const topology = await fetchNetworkTopology();
      console.log("Topology loaded:", topology);
      networkTopologyStore.setTopology(topology);

      // Build device state map for quick lookup
      deviceStates.clear();
      // Note: Device states would need to be fetched separately
      // For now, we'll extract link quality from edges
    } catch (error) {
      console.error("Failed to load network topology:", error);
      networkTopologyStore.setError(
        error instanceof Error ? error.message : "Unknown error"
      );
    }
  }

  async function handleRefresh() {
    if (refreshing) return;

    if (!showRefreshWarning) {
      showRefreshWarning = true;
      return;
    }

    try {
      refreshing = true;
      await refreshNetworkMap();

      // Wait a few seconds for zigbee2mqtt to process
      await new Promise((resolve) => setTimeout(resolve, 3000));

      await loadTopology();
      showRefreshWarning = false;
    } catch (error) {
      console.error("Failed to refresh network map:", error);
      alert(
        `Failed to refresh network map: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    } finally {
      refreshing = false;
    }
  }

  function cancelRefresh() {
    showRefreshWarning = false;
  }
</script>

<div class="network-map-view">
  <div class="header">
    <h2>Zigbee Network Map</h2>
    <div class="toolbar">
      <button
        class="btn btn-primary"
        onclick={handleRefresh}
        disabled={isRefreshing}
      >
        {#if refreshing}
          <span class="spinner"></span>
          Refreshing...
        {:else}
          🔄 Refresh Network Map
        {/if}
      </button>
    </div>
  </div>

  {#if showRefreshWarning}
    <div class="warning-banner">
      <div class="warning-content">
        <span class="warning-icon">⚠️</span>
        <div>
          <strong>Warning:</strong> Refreshing the network map will cause all devices
          to recalculate their routes. This may temporarily disrupt network communication.
        </div>
      </div>
      <div class="warning-actions">
        <button class="btn btn-danger" onclick={handleRefresh}>
          Proceed
        </button>
        <button class="btn btn-secondary" onclick={cancelRefresh}>
          Cancel
        </button>
      </div>
    </div>
  {/if}

  {#if $networkTopologyStore.loading}
    <div class="loading">
      <div class="spinner large"></div>
      <p>Loading network topology...</p>
    </div>
  {:else if $networkTopologyStore.error}
    <div class="error">
      <p>❌ Error: {$networkTopologyStore.error}</p>
      <button class="btn btn-primary" onclick={loadTopology}>Retry</button>
    </div>
  {:else if $networkTopologyStore.topology}
    <div class="stats">
      <div class="stat">
        <span class="stat-label">Devices:</span>
        <span class="stat-value"
          >{$networkTopologyStore.topology.devices.length}</span
        >
      </div>
      <div class="stat">
        <span class="stat-label">Connections:</span>
        <span class="stat-value"
          >{$networkTopologyStore.topology.edges.length}</span
        >
      </div>
      <div class="stat">
        <span class="stat-label">Routers:</span>
        <span class="stat-value">
          {$networkTopologyStore.topology.devices.filter((d) => d.is_bridge)
            .length}
        </span>
      </div>
    </div>

    <NetworkGraph {deviceStates} />

    <div class="legend">
      <h3>Legend</h3>
      <div class="legend-items">
        <div class="legend-item">
          <span class="legend-color" style="background-color: #f59e0b;"></span>
          <span>Coordinator</span>
        </div>
        <div class="legend-item">
          <span class="legend-color" style="background-color: #3b82f6;"></span>
          <span>Router</span>
        </div>
        <div class="legend-item">
          <span class="legend-color" style="background-color: #22c55e;"></span>
          <span>Excellent Link (LQI > 100)</span>
        </div>
        <div class="legend-item">
          <span class="legend-color" style="background-color: #eab308;"></span>
          <span>Good Link (LQI 50-100)</span>
        </div>
        <div class="legend-item">
          <span class="legend-color" style="background-color: #ef4444;"></span>
          <span>Poor Link (LQI &lt; 50)</span>
        </div>
      </div>
    </div>
  {:else}
    <div class="empty">
      <p>No network topology data available</p>
    </div>
  {/if}
</div>

<style>
  .network-map-view {
    padding: 20px;
    max-width: 1400px;
    margin: 0 auto;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
  }

  .toolbar {
    display: flex;
    gap: 12px;
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background-color: #3b82f6;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: #2563eb;
  }

  .btn-secondary {
    background-color: #6b7280;
    color: white;
  }

  .btn-secondary:hover {
    background-color: #4b5563;
  }

  .btn-danger {
    background-color: #ef4444;
    color: white;
  }

  .btn-danger:hover {
    background-color: #dc2626;
  }

  .warning-banner {
    background-color: #fef3c7;
    border: 1px solid #f59e0b;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .warning-content {
    display: flex;
    gap: 12px;
    flex: 1;
  }

  .warning-icon {
    font-size: 24px;
  }

  .warning-actions {
    display: flex;
    gap: 8px;
  }

  .stats {
    display: flex;
    gap: 24px;
    margin-bottom: 20px;
    padding: 16px;
    background: #f9fafb;
    border-radius: 8px;
  }

  .stat {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .stat-label {
    color: #6b7280;
    font-size: 14px;
  }

  .stat-value {
    font-size: 18px;
    font-weight: 600;
    color: #111827;
  }

  .legend {
    margin-top: 20px;
    padding: 16px;
    background: #f9fafb;
    border-radius: 8px;
  }

  .legend h3 {
    margin: 0 0 12px 0;
    font-size: 16px;
    font-weight: 600;
  }

  .legend-items {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
  }

  .legend-color {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    border: 1px solid #e5e7eb;
  }

  .loading,
  .error,
  .empty {
    text-align: center;
    padding: 60px 20px;
  }

  .loading p,
  .error p,
  .empty p {
    margin-top: 16px;
    color: #6b7280;
  }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .spinner.large {
    width: 32px;
    height: 32px;
    border-width: 3px;
    border-color: rgba(59, 130, 246, 0.3);
    border-top-color: #3b82f6;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
