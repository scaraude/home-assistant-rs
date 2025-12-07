<script lang="ts">
  import Router, { location } from 'svelte-spa-router';
  import SensorsView from './routes/SensorsView.svelte';
  import LogsView from './routes/LogsView.svelte';

  // Route definitions
  const routes = {
    '/': SensorsView,
    '/sensors': SensorsView,
    '/logs': LogsView,
  };

  // Track current route for active state
  $: currentPath = $location;
  $: isOnSensors = currentPath === '/' || currentPath === '/sensors';
  $: isOnLogs = currentPath === '/logs';
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
        <a
          href="#/"
          class="nav-button"
          class:active={isOnSensors}
        >
          Sensors
        </a>
        <a
          href="#/logs"
          class="nav-button"
          class:active={isOnLogs}
        >
          System Logs
        </a>
      </nav>
    </div>
  </header>

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
  }
</style>
