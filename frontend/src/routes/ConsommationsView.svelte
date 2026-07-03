<script lang="ts">
  import EnergyDashboard from "../lib/devices/EnergyDashboard.svelte";
</script>

<div class="consommations-view">
  <div class="board">
    <EnergyDashboard />
  </div>
</div>

<style>
  .consommations-view {
    width: 100%;
    height: 100%;
  }

  /* The gray "board" matches the sensor explorer: content sits on it as white
     panels with a tight gutter, so the energy page reads as part of the site. */
  .board {
    padding: 1.25rem;
    background: #eef1f5;
    border-radius: 18px;
    margin: 1.25rem;
    min-height: calc(100vh - var(--app-header-height, 88px) - 2.5rem);
  }

  @media (max-width: 768px) {
    .board {
      padding: 0.75rem;
      margin: 0.75rem;
      border-radius: 14px;
    }
  }
</style>
