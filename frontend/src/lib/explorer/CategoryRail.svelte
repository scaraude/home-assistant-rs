<script lang="ts">
  import { slide } from "svelte/transition";
  import { sensorsMemory } from "../memory";
  import { explorerConfig } from "./explorerStore";
  import {
    CATEGORIES,
    devicesForCategory,
    formatMetricValue,
    metricValue,
    seriesKey,
    type ExplorerMetric,
  } from "./metrics";
  import type { SensorReading } from "../api";

  // Multiple sections can be open at once. Température is open by default.
  let open = $state<Set<ExplorerMetric>>(new Set<ExplorerMetric>(["temperature"]));

  const devices = $derived($sensorsMemory.devices);
  const latestByDevice = $derived($sensorsMemory.latestByDevice);
  const selected = $derived($explorerConfig.series);
  const selectedKeys = $derived(new Set(selected.map((s) => seriesKey(s.deviceId, s.metric))));

  const countByCategory = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const s of selected) counts[s.metric] = (counts[s.metric] ?? 0) + 1;
    return counts;
  });

  function toggleSection(id: ExplorerMetric) {
    const next = new Set(open);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    open = next;
  }

  function isSelected(deviceId: string, metric: ExplorerMetric) {
    return selectedKeys.has(seriesKey(deviceId, metric));
  }

  function allSelected(list: { device_id: string }[], metric: ExplorerMetric) {
    return list.length > 0 && list.every((d) => isSelected(d.device_id, metric));
  }

  function toggleAll(
    list: { device_id: string; color: string | null }[],
    metric: ExplorerMetric,
    event: MouseEvent,
  ) {
    // Don't let the click bubble to the header (which expands/collapses).
    event.stopPropagation();
    if (allSelected(list, metric)) {
      explorerConfig.clearCategory(metric);
    } else {
      explorerConfig.selectCategory(
        metric,
        list.map((d) => ({ deviceId: d.device_id, color: d.color })),
      );
    }
  }

  function latestLabel(deviceId: string, metric: ExplorerMetric): string | null {
    const reading = latestByDevice[deviceId] as SensorReading | null | undefined;
    if (!reading) return null;
    const v = metricValue(reading, metric);
    return v === null ? null : formatMetricValue(v, metric);
  }

  function swatchColor(deviceId: string, metric: ExplorerMetric, fallback: string | null): string {
    const s = selected.find((x) => x.deviceId === deviceId && x.metric === metric);
    return s?.color ?? fallback ?? "#9ca3af";
  }
</script>

<div class="accordion" role="group" aria-label="Catégories de capteurs">
  {#each CATEGORIES as cat, i (cat.id)}
    {@const isOpen = open.has(cat.id)}
    {@const list = devicesForCategory(devices, cat.id)}
    <section class="section" class:open={isOpen} class:first={i === 0}>
      <div class="head">
        <button
          class="head-toggle"
          aria-expanded={isOpen}
          onclick={() => toggleSection(cat.id)}
        >
          <svg class="icon" viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
            <path d={cat.iconPath} />
          </svg>
          <span class="title">{cat.label}</span>
          {#if countByCategory[cat.id]}
            <span class="badge">{countByCategory[cat.id]}</span>
          {/if}
        </button>
        {#if list.length > 0}
          <button
            class="select-all"
            class:on={allSelected(list, cat.id)}
            onclick={(e) => toggleAll(list, cat.id, e)}
            title={allSelected(list, cat.id)
              ? "Tout désélectionner"
              : "Tout sélectionner"}
          >
            {allSelected(list, cat.id) ? "Aucun" : "Tout"}
          </button>
        {/if}
        <button
          class="chev-btn"
          aria-label={isOpen ? "Replier" : "Déplier"}
          aria-expanded={isOpen}
          onclick={() => toggleSection(cat.id)}
        >
          <svg class="chevron" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
      </div>

      {#if isOpen}
        <div class="body" transition:slide={{ duration: 180 }}>
          {#if list.length === 0}
            <p class="empty">Aucun capteur.</p>
          {:else}
            {#each list as device (device.device_id)}
              {@const sel = isSelected(device.device_id, cat.id)}
              {@const value = latestLabel(device.device_id, cat.id)}
              <button
                class="row"
                class:selected={sel}
                onclick={() => explorerConfig.toggleSeries(device.device_id, cat.id, device.color)}
                aria-pressed={sel}
              >
                <span
                  class="dot"
                  class:filled={sel}
                  style={sel
                    ? `background:${swatchColor(device.device_id, cat.id, device.color)};border-color:${swatchColor(device.device_id, cat.id, device.color)}`
                    : ""}
                ></span>
                <span class="name">{device.name}</span>
                {#if value}<span class="value">{value}</span>{/if}
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </section>
  {/each}
</div>

<style>
  .accordion {
    display: flex;
    flex-direction: column;
    background: white;
    border-radius: 12px;
    overflow: hidden;
    box-shadow:
      0 1px 2px rgba(16, 24, 40, 0.04),
      0 1px 3px rgba(16, 24, 40, 0.06);
    height: 100%;
  }

  .section {
    border-top: 1px solid #f1f3f5;
  }

  .section.first {
    border-top: none;
  }

  .head {
    display: flex;
    align-items: center;
  }

  .head-toggle {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.875rem 0.5rem 0.875rem 1rem;
    background: transparent;
    border: none;
    cursor: pointer;
    color: #667085;
    font-size: 0.9375rem;
    font-weight: 600;
    text-align: left;
    transition: color 0.15s;
  }

  .head-toggle:hover {
    color: #101828;
  }

  .section.open .head-toggle {
    color: #101828;
  }

  .select-all {
    flex-shrink: 0;
    padding: 0.2rem 0.55rem;
    border: 1px solid #e4e7ec;
    background: #fff;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 600;
    color: #667085;
    cursor: pointer;
    transition: all 0.12s;
  }

  .select-all:hover {
    border-color: #b2ccff;
    color: #1d4ed8;
  }

  .select-all.on {
    background: #eff6ff;
    border-color: #bfdbfe;
    color: #1d4ed8;
  }

  .chev-btn {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    padding: 0.875rem 1rem 0.875rem 0.5rem;
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .icon {
    color: #98a2b3;
    flex-shrink: 0;
    transition: color 0.15s;
  }

  .section.open .icon {
    color: #3b82f6;
  }

  .title {
    flex: 1;
  }

  .badge {
    min-width: 1.25rem;
    height: 1.25rem;
    padding: 0 0.375rem;
    border-radius: 999px;
    background: #eff6ff;
    color: #1d4ed8;
    font-size: 0.75rem;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .chevron {
    color: #98a2b3;
    transition: transform 0.18s ease;
  }

  .section.open .chevron {
    transform: rotate(180deg);
  }

  .body {
    padding: 0 0.5rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .empty {
    color: #98a2b3;
    font-size: 0.8125rem;
    padding: 0.25rem 0.5rem 0.5rem;
    margin: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.625rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.12s;
  }

  .row:hover {
    background: #f7f8fa;
  }

  .row.selected {
    background: #f2f7ff;
  }

  .dot {
    width: 14px;
    height: 14px;
    border-radius: 5px;
    border: 2px solid #d0d5dd;
    flex-shrink: 0;
    box-sizing: border-box;
    transition: all 0.12s;
  }

  .name {
    flex: 1;
    font-size: 0.875rem;
    color: #475467;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row.selected .name {
    color: #101828;
    font-weight: 500;
  }

  .value {
    font-size: 0.8125rem;
    color: #98a2b3;
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 768px) {
    .accordion {
      height: auto;
    }
  }
</style>
