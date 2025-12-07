<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import SensorCard from './lib/SensorCard.svelte';
  import LogsPage from './lib/LogsPage.svelte';
  import { fetchAllSensorData, type SensorData } from './lib/api';

  type View = 'sensors' | 'logs';

  let currentView: View = 'sensors';
  let sensorData: SensorData[] = [];
  let loading = true;
  let error: string | null = null;
  let intervalId: number | null = null;

  async function loadData() {
    try {
      error = null;
      sensorData = await fetchAllSensorData(24);
      loading = false;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load sensor data';
      loading = false;
    }
  }

  function startPolling() {
    // Initial load
    loadData();

    // Poll every 15 seconds
    intervalId = window.setInterval(() => {
      loadData();
    }, 15000);
  }

  function stopPolling() {
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  onMount(() => {
    startPolling();
  });

  onDestroy(() => {
    stopPolling();
  });
</script>

<main>
  <header>
    <div class="header-content">
      <div class="logo">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/>
        </svg>
      </div>
      <h1>Home Assistant</h1>
      <nav class="nav-buttons">
        <button
          class="nav-button"
          class:active={currentView === 'sensors'}
          on:click={() => currentView = 'sensors'}
        >
          Sensors
        </button>
        <button
          class="nav-button"
          class:active={currentView === 'logs'}
          on:click={() => currentView = 'logs'}
        >
          System Logs
        </button>
      </nav>
    </div>
  </header>

  <div class="container">
    {#if currentView === 'sensors'}
      {#if loading}
        <div class="loading">
          <div class="spinner"></div>
          <p>Loading sensors...</p>
        </div>
      {:else if error}
        <div class="error">
          <p>Error: {error}</p>
          <button on:click={loadData}>Retry</button>
        </div>
      {:else if sensorData.length === 0}
        <div class="no-sensors">
          <p>No sensors found</p>
        </div>
      {:else}
        <div class="sensor-grid">
          {#each sensorData as sensor (sensor.id)}
            <SensorCard sensorData={sensor} />
          {/each}
        </div>
      {/if}
    {:else if currentView === 'logs'}
      <LogsPage />
    {/if}
  </div>
</main>

<style>
  main {
    min-height: 100vh;
    background: #f3f4f6;
  }

  header {
    background: white;
    border-bottom: 1px solid #e5e7eb;
    padding: 1rem 0;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .header-content {
    max-width: 1400px;
    margin: 0 auto;
    padding: 0 1rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .nav-buttons {
    margin-left: auto;
    display: flex;
    gap: 0.5rem;
  }

  .nav-button {
    padding: 0.5rem 1rem;
    background: transparent;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.2s;
  }

  .nav-button:hover {
    background: #f9fafb;
    color: #111827;
    border-color: #d1d5db;
  }

  .nav-button.active {
    background: #3b82f6;
    color: white;
    border-color: #3b82f6;
  }

  .logo {
    width: 36px;
    height: 36px;
    background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }

  .logo svg {
    width: 24px;
    height: 24px;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #111827;
  }

  .container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 0;
    color: #6b7280;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 0;
    color: #dc2626;
  }

  .error button {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background 0.2s;
  }

  .error button:hover {
    background: #2563eb;
  }

  .no-sensors {
    text-align: center;
    padding: 4rem 0;
    color: #6b7280;
  }

  .sensor-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.5rem;
  }

  @media (max-width: 640px) {
    .sensor-grid {
      grid-template-columns: 1fr;
    }

    h1 {
      font-size: 1.25rem;
    }

    .nav-buttons {
      position: fixed;
      bottom: 0;
      left: 0;
      right: 0;
      background: white;
      border-top: 1px solid #e5e7eb;
      padding: 0.75rem;
      margin-left: 0;
      gap: 0.75rem;
      box-shadow: 0 -2px 10px rgba(0, 0, 0, 0.1);
      z-index: 100;
    }

    .nav-button {
      flex: 1;
      padding: 0.75rem;
    }

    .container {
      padding-bottom: 5rem;
    }
  }
</style>
