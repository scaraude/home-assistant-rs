<script lang="ts">
  import { onMount } from "svelte";
  import SensorExplorer from "../lib/explorer/SensorExplorer.svelte";
  import PageState from "../lib/shared/PageState.svelte";
  import { sensorsMemory } from "../lib/memory";

  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadData(force = false) {
    const state = $sensorsMemory;
    if (!force && state.devicesLoaded) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      await sensorsMemory.ensureDevices(force);
    } catch (err) {
      error = err instanceof Error ? err.message : "Échec du chargement des capteurs";
    } finally {
      loading = false;
    }
  }

  function retryLoad() {
    void loadData(true);
  }

  onMount(() => {
    void loadData();
  });
</script>

<div class="sensor-view">
  <PageState {loading} {error} loadingText="Chargement des capteurs…" onRetry={retryLoad}>
    <SensorExplorer />
  </PageState>
</div>

<style>
  .sensor-view {
    width: 100%;
    height: 100%;
  }
</style>
