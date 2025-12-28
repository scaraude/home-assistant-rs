<script lang="ts">
  import { onMount } from 'svelte';
  import Router, { location } from 'svelte-spa-router';
  import SensorsView from './routes/SensorsView.svelte';
  import LogsView from './routes/LogsView.svelte';
  import CommanderView from './routes/CommanderView.svelte';
  import { fetchSensors, fetchReadings, fetchSwitches } from './lib/api';
  import { dataCache } from './lib/stores/dataCache';
  import { eventStream, type SystemEvent } from './lib/websocket';

  // Route definitions
  const routes = {
    '/': SensorsView,
    '/sensors': SensorsView,
    '/commander': CommanderView,
    '/logs': LogsView,
  };

  const DEFAULT_SENSOR_HOURS = 24;
  let initializing = $state(true);
  let initError = $state<string | null>(null);
  let initialLoadInFlight = $state(false);

  async function loadInitialData() {
    if (initialLoadInFlight) {
      return;
    }

    initialLoadInFlight = true;
    initializing = true;
    initError = null;

    try {
      const [sensors, readingsResult, switches] = await Promise.all([
        fetchSensors(),
        fetchReadings(undefined, DEFAULT_SENSOR_HOURS),
        fetchSwitches(),
      ]);

      dataCache.setSensors(sensors);
      dataCache.setSensorReadings(
        readingsResult.readings,
        readingsResult.latestTimestamp ?? 0,
        DEFAULT_SENSOR_HOURS,
      );
      dataCache.setSwitches(switches);
    } catch (error) {
      console.error('Failed to load initial data:', error);
      initError = error instanceof Error ? error.message : 'Failed to load initial data';
    } finally {
      initializing = false;
      initialLoadInFlight = false;
    }
  }

  function normalizeSwitchState(state: boolean | string): boolean {
    if (typeof state === 'string') {
      return state.toUpperCase() === 'ON';
    }
    return state;
  }

  function handleEvent(event: SystemEvent) {
    switch (event.event) {
      case 'sensor_reading':
        dataCache.mergeSensorReading(event.reading);
        break;
      case 'switch_state':
        dataCache.updateSwitchState(event.device_id, {
          state: normalizeSwitchState(event.state),
          last_seen: event.timestamp,
        });
        dataCache.updateDeviceState(event.device_id, { last_seen: event.timestamp });
        break;
      case 'device_state':
        dataCache.updateDeviceState(event.device_id, {
          battery_level: event.battery,
          link_quality: event.link_quality,
          last_seen: event.timestamp,
        });
        break;
      default:
        break;
    }
  }

  function retryInitialLoad() {
    void loadInitialData();
  }

  onMount(() => {
    let unsubscribe: (() => void) | undefined;

    void loadInitialData();
    eventStream.connect();
    unsubscribe = eventStream.events.subscribe((evt) => {
      if (!evt) {
        return;
      }
      handleEvent(evt);
    });

    return () => {
      unsubscribe?.();
      eventStream.disconnect();
    };
  });

  // Track current route for active state
  let currentPath = $derived($location);
  let isOnSensors = $derived(currentPath === '/' || currentPath === '/sensors');
  let isOnCommander = $derived(currentPath === '/commander');
  let isOnLogs = $derived(currentPath === '/logs');
</script>

<main>
  <header>
    <div class="header-content">
      <div class="logo">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/>
        </svg>
      </div>
      <h1>Home Automation</h1>

      <nav class="nav-buttons">
        <a
          href="#/"
          class="nav-button"
          class:active={isOnSensors}
        >
          Sensors
        </a>
        <a
          href="#/commander"
          class="nav-button"
          class:active={isOnCommander}
        >
          Commander
        </a>
        <a
          href="#/logs"
          class="nav-button"
          class:active={isOnLogs}
        >
          System Logs
        </a>
      </nav>

      {#if initializing}
        <div class="sync-indicator">
          <span class="dot"></span>
          Syncing data...
        </div>
      {/if}
    </div>
  </header>

  {#if initError}
    <div class="init-error">
      <span>Failed to load initial data: {initError}</span>
      <button type="button" onclick={retryInitialLoad}>Retry</button>
    </div>
  {/if}

  <div class="container">
    <Router {routes} />
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
    display: flex;
    gap: 0.5rem;
    margin-left: auto;
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
    text-decoration: none;
    display: inline-block;
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

  .sync-indicator {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8125rem;
    color: #2563eb;
    margin-left: 1rem;
  }

  .sync-indicator .dot {
    width: 8px;
    height: 8px;
    border-radius: 9999px;
    background: #2563eb;
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 1;
    }
  }

  .init-error {
    max-width: 1400px;
    margin: 1rem auto 0;
    padding: 0.75rem 1rem;
    background: #fee2e2;
    border: 1px solid #fecaca;
    color: #b91c1c;
    border-radius: 6px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .init-error button {
    background: #dc2626;
    border: none;
    color: white;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8125rem;
  }

  .init-error button:hover {
    background: #b91c1c;
  }

  @media (max-width: 640px) {
    h1 {
      font-size: 1.25rem;
    }

    .header-content {
      flex-wrap: wrap;
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

    .init-error {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
