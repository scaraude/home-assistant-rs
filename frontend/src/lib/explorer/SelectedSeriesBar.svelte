<script lang="ts">
  import { sensorsMemory } from "../memory";
  import { explorerConfig, type ExplorerSeries } from "./explorerStore";
  import { CATEGORY_BY_ID } from "./metrics";

  let { series = [] }: { series?: ExplorerSeries[] } = $props();

  const deviceMap = $derived(new Map($sensorsMemory.devices.map((d) => [d.device_id, d])));

  function name(deviceId: string) {
    return deviceMap.get(deviceId)?.name ?? deviceId;
  }
</script>

{#if series.length > 0}
  <div class="chips">
    {#each series as s (s.deviceId + ":" + s.metric)}
      {@const cat = CATEGORY_BY_ID[s.metric]}
      <span class="chip">
        <label class="swatch" style={`background:${s.color}`} title="Changer la couleur">
          <input
            type="color"
            value={s.color}
            oninput={(e) => explorerConfig.setColor(s.deviceId, s.metric, e.currentTarget.value)}
          />
        </label>
        <span class="text">{name(s.deviceId)} · {cat.label}{cat.unit ? ` (${cat.unit})` : ""}</span>
        <button
          class="remove"
          aria-label={`Retirer ${name(s.deviceId)} ${cat.label}`}
          onclick={() => explorerConfig.removeSeries(s.deviceId, s.metric)}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </span>
    {/each}

    <button class="clear" onclick={() => explorerConfig.clear()}>Tout effacer</button>
  </div>
{/if}

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.35rem 0.3rem 0.4rem;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 999px;
    font-size: 0.8125rem;
    color: #374151;
  }

  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    cursor: pointer;
    position: relative;
    overflow: hidden;
    flex-shrink: 0;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
  }

  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    border: none;
    padding: 0;
  }

  .text {
    white-space: nowrap;
  }

  .remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: #9ca3af;
    cursor: pointer;
  }

  .remove:hover {
    background: #f3f4f6;
    color: #ef4444;
  }

  .clear {
    background: transparent;
    border: none;
    color: #6b7280;
    font-size: 0.8125rem;
    cursor: pointer;
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
  }

  .clear:hover {
    background: #f3f4f6;
    color: #111827;
  }
</style>
