<script lang="ts">
  import { onMount } from 'svelte';
  import SwitchCard from '../lib/devices/SwitchCard.svelte';
  import PageState from '../lib/shared/PageState.svelte';
  import { fetchAutomationRules } from '../lib/api';
  import { automationStore } from '../lib/stores/automations';
  import { switchesMemory } from '../lib/memory';

  let loading = $state(true);
  let error = $state<string | null>(null);
  let isFetching = $state(false);

  let switches = $derived($switchesMemory.devices);

  async function loadSwitches(force = false) {
    if (isFetching) return;
    if (!force && $switchesMemory.loaded) {
      loading = false;
      return;
    }

    isFetching = true;
    loading = true;
    error = null;

    try {
      await switchesMemory.ensureSwitches(force);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load switches';
      console.error('Failed to load switches:', err);
    } finally {
      isFetching = false;
      loading = false;
    }
  }

  async function loadAutomationRules() {
    try {
      const rules = await fetchAutomationRules();
      automationStore.setRules(rules);
    } catch (err) {
      console.warn('Failed to load automation rules:', err);
    }
  }

  onMount(() => {
    void loadSwitches(!$switchesMemory.loaded);
    void loadAutomationRules();
  });
</script>

<div class="commander-view">
  <div class="commander-header">
    <h2>Commander</h2>
    <p class="header-description">Control your smart switches</p>
  </div>

  <PageState
    {loading}
    {error}
    empty={switches.length === 0}
    loadingText="Loading switches..."
    emptyTitle="No switches found"
    onRetry={() => loadSwitches(true)}
  >
    {#snippet emptyState()}
      <p>Add switches to your Zigbee2MQTT setup to control them here.</p>
    {/snippet}
    <div class="switches-grid">
      {#each switches as device (device.id)}
        <SwitchCard {device} />
      {/each}
    </div>
  </PageState>
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

  .commander-header h2 {
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

  .switches-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.5rem;
  }

  @media (max-width: 640px) {
    .switches-grid {
      grid-template-columns: 1fr;
      gap: var(--card-gap, 1rem);
    }

    .commander-header {
      margin-bottom: 1.5rem;
    }

    .commander-header h2 {
      font-size: 1.25rem;
    }
  }

  @media (max-width: 480px) {
    .commander-header {
      margin-bottom: 1rem;
      padding-bottom: 0.75rem;
    }

    .commander-header h2 {
      font-size: 1.125rem;
    }

    .header-description {
      font-size: 0.8125rem;
    }

    .switches-grid {
      gap: var(--card-gap, 0.75rem);
    }
  }
</style>
