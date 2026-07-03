<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { formatDistanceToNow } from "date-fns";
  import { fr } from "date-fns/locale";
  import { deviceStateMemory, sensorsMemory } from "../memory";
  import type { EnergySensorReading } from "../api/sensors";
  import type { DeviceInfo, DeviceState } from "../types/devices";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import EditableDeviceName from "../devices/EditableDeviceName.svelte";
  import EnergyDashboard from "../devices/EnergyDashboard.svelte";
  import { URI_FRONTEND } from "../shared/constant/URI";

  interface Props {
    params: { id: string };
  }

  let { params }: Props = $props();

  let deviceState = $state<DeviceState | null>(null);
  let editingName = $state(false);
  let device = $state<DeviceInfo | null>(null);
  let triedLoad = $state(false);

  // Latch the device once it appears in the store. We don't derive it directly:
  // the store's IndexedDB rehydration can transiently reset `devices` to empty,
  // and we don't want that to blank an already-loaded page.
  $effect(() => {
    const found = $sensorsMemory.devices.find((d) => d.device_id === params.id);
    if (found) device = found;
  });
  const notFound = $derived(triedLoad && !device);

  const latestReading = $derived.by(() => {
    const reading = $sensorsMemory.latestByDevice[params.id] ?? null;
    if (!reading || reading.type !== "energy_meter") return null;
    return reading as EnergySensorReading;
  });

  const powerMagnitude = $derived(
    latestReading?.power !== undefined ? Math.abs(latestReading.power) : null,
  );
  const isProducing = $derived((latestReading?.power ?? 0) < 0);

  const deviceName = $derived(device?.name ?? params.id);

  onMount(() => {
    void sensorsMemory.ensureDevices().finally(() => {
      triedLoad = true;
    });
    void sensorsMemory.refreshLatestReadings();
    // Battery/signal badges are secondary — fetch without gating the page.
    void deviceStateMemory
      .ensureDeviceState(params.id)
      .then((state) => {
        if (state) deviceState = state;
      })
      .catch(() => {});
  });

  function goBack() {
    push(URI_FRONTEND.CONSOMMATIONS);
  }

  function startEditingName() {
    editingName = true;
  }
  function handleNameSaved() {
    editingName = false;
    // `device` and `deviceName` are derived from the store, which the rename
    // already updated — nothing to reassign here.
  }

  function lastSeen(): string {
    const date = deviceState?.last_seen || latestReading?.timestamp;
    // An invalid Date is still truthy, so guard on the time value too.
    if (!date || Number.isNaN(date.getTime())) return "—";
    const diff = Date.now() - date.getTime();
    if (diff < 60_000) return "à l'instant";
    return formatDistanceToNow(date, { addSuffix: true, locale: fr });
  }

  const nf = (v: number | undefined, d = 1) =>
    v === undefined
      ? "—"
      : v.toLocaleString("fr-FR", { minimumFractionDigits: d, maximumFractionDigits: d });

  function powerText(w: number | null): { value: string; unit: string } {
    if (w === null) return { value: "—", unit: "W" };
    return Math.abs(w) >= 1000
      ? { value: nf(w / 1000, 2), unit: "kW" }
      : { value: String(Math.round(w)), unit: "W" };
  }
  const powerDisplay = $derived(powerText(powerMagnitude));
</script>

<div class="energy-detail">
  <div class="board">
    <header class="detail-header">
      <button class="back-button" onclick={goBack} type="button">
        <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
          <path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z" />
        </svg>
        <span>Consommations</span>
      </button>
    </header>

    {#if notFound}
      <div class="panel state-panel">
        <p class="state-msg">Compteur d'énergie introuvable</p>
        <button class="retry" onclick={goBack} type="button">Retour</button>
      </div>
    {:else if !device}
      <div class="panel state-panel">
        <p class="state-msg">Chargement du compteur…</p>
      </div>
    {:else}
      <!-- Identity -->
      <section class="panel identity">
        <div class="identity-icon">
          <svg viewBox="0 0 24 24" fill="currentColor" width="26" height="26">
            <path d="M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.07-.12C8.48 10.94 10.42 7.54 13 3h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z" />
          </svg>
        </div>
        <div class="identity-details">
          <h1 class="device-name">
            <EditableDeviceName
              deviceId={params.id}
              name={deviceName}
              isEdit={editingName}
              onSaved={handleNameSaved}
              class="device-name-text"
            />
            {#if !editingName}
              <button class="edit-trigger" onclick={startEditingName} title="Renommer" type="button">
                <svg viewBox="0 0 24 24" fill="currentColor" width="15" height="15">
                  <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z" />
                </svg>
              </button>
            {/if}
          </h1>
          <div class="identity-meta">
            <span class="device-id">{params.id}</span>
            <span class="dot-sep">•</span>
            <span>{lastSeen()}</span>
          </div>
        </div>
        <div class="badges">
          {#if deviceState?.battery_level != null}
            <StatusBadge type="battery" value={deviceState.battery_level} />
          {/if}
          {#if deviceState?.link_quality != null}
            <StatusBadge type="signal" value={deviceState.link_quality} />
          {/if}
        </div>
      </section>

      <!-- Live now -->
      <section class="panel live">
        <div class="live-power">
          <span class="live-label">Puissance en direct</span>
          <div class="power-value">
            <span class="pv-number">{powerDisplay.value}</span>
            <span class="pv-unit">{powerDisplay.unit}</span>
          </div>
          <span class="flow" class:producing={isProducing}>
            {isProducing ? "↑ Production" : "↓ Consommation"}
          </span>
        </div>
        <div class="live-metrics">
          <div class="metric">
            <span class="m-value">{nf(latestReading?.voltage, 1)}<span class="m-unit">V</span></span>
            <span class="m-label">Tension</span>
          </div>
          <div class="metric">
            <span class="m-value">{nf(latestReading?.current, 2)}<span class="m-unit">A</span></span>
            <span class="m-label">Courant</span>
          </div>
          <div class="metric">
            <span class="m-value">{nf(latestReading?.ac_frequency, 1)}<span class="m-unit">Hz</span></span>
            <span class="m-label">Fréquence</span>
          </div>
          <div class="metric">
            <span class="m-value">{nf(latestReading?.power_factor, 2)}</span>
            <span class="m-label">Facteur de puissance</span>
          </div>
        </div>
      </section>

      <!-- Cumulative dashboard, scoped to this meter -->
      <EnergyDashboard deviceId={params.id} />
    {/if}
  </div>
</div>

<style>
  .energy-detail {
    width: 100%;
    height: 100%;
  }
  .board {
    padding: 1.25rem;
    background: #eef1f5;
    border-radius: 18px;
    margin: 1.25rem;
    min-height: calc(100vh - var(--app-header-height, 88px) - 2.5rem);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .detail-header {
    display: flex;
  }
  .back-button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.9rem;
    background: #fff;
    border: 1px solid var(--color-card-border);
    border-radius: 9px;
    font-size: 0.8125rem;
    font-weight: 600;
    color: #667085;
    cursor: pointer;
    transition: all 0.15s;
    box-shadow: var(--shadow-sm);
  }
  .back-button:hover {
    color: #101828;
  }

  .panel {
    background: var(--color-card-bg);
    border-radius: 14px;
    box-shadow: var(--shadow-sm);
  }

  /* Identity */
  .identity {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.1rem 1.25rem;
  }
  .identity-icon {
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    border-radius: 12px;
    display: grid;
    place-items: center;
    color: #d97706;
    background: linear-gradient(135deg, #fffbeb, #fef3c7);
    border: 1px solid #fde68a;
  }
  .identity-details {
    flex: 1;
    min-width: 0;
  }
  .device-name {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0 0 0.15rem;
    font-size: 1.25rem;
    font-weight: 700;
    color: #101828;
  }
  :global(.device-name-text) {
    font-size: 1.25rem;
    font-weight: 700;
    color: #101828;
  }
  .edit-trigger {
    padding: 0.3rem;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #98a2b3;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: all 0.15s;
  }
  .edit-trigger:hover {
    background: #f2f4f7;
    color: #667085;
  }
  .identity-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: #98a2b3;
  }
  .device-id {
    font-family: var(--font-mono);
    font-size: 0.72rem;
  }
  .dot-sep {
    opacity: 0.5;
  }
  .badges {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  /* Live */
  .live {
    display: flex;
    align-items: stretch;
    gap: 1.25rem;
    padding: 1.25rem;
    flex-wrap: wrap;
  }
  .live-power {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    justify-content: center;
    min-width: 150px;
  }
  .live-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: #667085;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .power-value {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }
  .pv-number {
    font-size: 2.6rem;
    line-height: 1;
    font-weight: 800;
    letter-spacing: -0.025em;
    color: #101828;
    font-variant-numeric: tabular-nums;
  }
  .pv-unit {
    font-size: 1rem;
    font-weight: 600;
    color: #98a2b3;
  }
  .flow {
    width: fit-content;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    background: #eff6ff;
    color: #1d4ed8;
  }
  .flow.producing {
    background: #ecfdf3;
    color: #16a34a;
  }
  .live-metrics {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.75rem;
    min-width: 0;
  }
  .metric {
    background: #f9fafb;
    border: 1px solid #f1f3f5;
    border-radius: 10px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    align-items: flex-start;
  }
  .m-value {
    font-size: 1.15rem;
    font-weight: 700;
    color: #101828;
    font-variant-numeric: tabular-nums;
  }
  .m-unit {
    font-size: 0.72rem;
    font-weight: 600;
    color: #98a2b3;
    margin-left: 0.15rem;
  }
  .m-label {
    font-size: 0.72rem;
    color: #98a2b3;
  }

  /* States */
  .state-panel {
    padding: 2.5rem 1.25rem;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.85rem;
  }
  .state-msg {
    margin: 0;
    color: #667085;
  }
  .retry {
    padding: 0.5rem 1.1rem;
    border: 1px solid var(--color-card-border);
    background: #fff;
    border-radius: 8px;
    font-weight: 600;
    color: #101828;
    cursor: pointer;
  }

  @media (max-width: 760px) {
    .board {
      padding: 0.75rem;
      margin: 0.75rem;
    }
    .live-metrics {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
