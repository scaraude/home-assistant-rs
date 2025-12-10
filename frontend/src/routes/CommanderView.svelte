<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import SwitchCard from '../lib/SwitchCard.svelte';
  import { fetchSwitches, type SwitchDevice } from '../lib/api';

  let switches: SwitchDevice[] = [];
  let loading = true;
  let error: string | null = null;
  let intervalId: number | null = null;

  async function loadSwitches() {
    try {
      error = null;
      switches = await fetchSwitches();
      loading = false;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load switches';
      loading = false;
      console.error('Failed to load switches:', err);
    }
  }

  function startPolling() {
    // Initial load
    loadSwitches();

    // Poll every 15 seconds to get updated state
    intervalId = window.setInterval(() => {
      // Don't show loading on subsequent polls
      fetchSwitches()
        .then(newSwitches => {
          switches = newSwitches;
          error = null;
        })
        .catch(err => {
          console.error('Failed to poll switches:', err);
          // Don't update error state on polling failures to avoid UI flicker
        });
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

<div class="commander-view">
  <div class="commander-header">
    <div class="header-content">
      <h2>Commander</h2>
      <p class="header-description">Control your smart switches</p>
    </div>
  </div>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>Loading switches...</p>
    </div>
  {:else if error}
    <div class="error">
      <div class="error-icon">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
        </svg>
      </div>
      <p>Error: {error}</p>
      <button on:click={loadSwitches}>Retry</button>
    </div>
  {:else if switches.length === 0}
    <div class="no-switches">
      <div class="no-switches-icon">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M9 21c0 .55.45 1 1 1h4c.55 0 1-.45 1-1v-1H9v1zm3-19C8.14 2 5 5.14 5 9c0 2.38 1.19 4.47 3 5.74V17c0 .55.45 1 1 1h6c.55 0 1-.45 1-1v-2.26c1.81-1.27 3-3.36 3-5.74 0-3.86-3.14-7-7-7z"/>
        </svg>
      </div>
      <h3>No switches found</h3>
      <p>Add switches to your Zigbee2MQTT setup to control them here.</p>
    </div>
  {:else}
    <div class="switches-grid">
      {#each switches as device (device.id)}
        <SwitchCard {device} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .commander-view {
    width: 100%;
  }

  .commander-header {
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-content h2 {
    margin: 0 0 0.25rem 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #111827;
  }

  .header-description {
    margin: 0;
    font-size: 0.875rem;
    color: #6b7280;
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

  .loading p {
    margin: 0;
    font-size: 0.9375rem;
  }

  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 0;
    color: #dc2626;
  }

  .error-icon {
    width: 60px;
    height: 60px;
    color: #dc2626;
    margin-bottom: 1rem;
  }

  .error-icon svg {
    width: 100%;
    height: 100%;
  }

  .error p {
    margin: 0 0 1rem 0;
    font-size: 0.9375rem;
  }

  .error button {
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

  .no-switches {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
    color: #6b7280;
  }

  .no-switches-icon {
    width: 80px;
    height: 80px;
    color: #d1d5db;
    margin-bottom: 1.5rem;
  }

  .no-switches-icon svg {
    width: 100%;
    height: 100%;
  }

  .no-switches h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .no-switches p {
    margin: 0;
    font-size: 0.9375rem;
    max-width: 400px;
  }

  .switches-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.5rem;
  }

  @media (max-width: 640px) {
    .switches-grid {
      grid-template-columns: 1fr;
    }

    .commander-header h2 {
      font-size: 1.25rem;
    }

    .no-switches {
      padding: 3rem 1rem;
    }
  }
</style>
