<script lang="ts">
  import {
    startOfDay,
    addDays,
    startOfWeek,
    addWeeks,
    startOfMonth,
    addMonths,
    format,
  } from "date-fns";
  import { fr } from "date-fns/locale";
  import { fetchEnergySummary, type EnergySummary } from "../api/energy";
  import type { DeviceInfo } from "../types/devices";
  import { sensorsMemory } from "../memory";

  type Scope = "day" | "week" | "month";

  let { deviceId }: { deviceId?: string } = $props();

  // ---- period selection ----------------------------------------------------
  let scope = $state<Scope>("month");
  let offset = $state(0); // 0 = current period, -1 = previous, …

  interface Period {
    start: Date;
    end: Date;
    bucketSeconds: number;
    label: string;
    subLabel: "heure" | "jour";
  }

  function periodFor(s: Scope, o: number): Period {
    const now = new Date();
    if (s === "day") {
      const start = addDays(startOfDay(now), o);
      return {
        start,
        end: addDays(start, 1),
        bucketSeconds: 3600,
        label: format(start, "EEE d MMMM", { locale: fr }),
        subLabel: "heure",
      };
    }
    if (s === "week") {
      const start = addWeeks(startOfWeek(now, { weekStartsOn: 1 }), o);
      const last = addDays(start, 6);
      return {
        start,
        end: addWeeks(start, 1),
        bucketSeconds: 86400,
        label: `${format(start, "d MMM", { locale: fr })} – ${format(last, "d MMM", { locale: fr })}`,
        subLabel: "jour",
      };
    }
    const start = addMonths(startOfMonth(now), o);
    return {
      start,
      end: addMonths(start, 1),
      bucketSeconds: 86400,
      label: format(start, "MMMM yyyy", { locale: fr }),
      subLabel: "jour",
    };
  }

  const period = $derived(periodFor(scope, offset));
  const atCurrent = $derived(offset >= 0);

  function setScope(s: Scope) {
    scope = s;
    offset = 0;
  }

  // ---- data ----------------------------------------------------------------
  let summary = $state<EnergySummary | null>(null);
  let previousTotal = $state<number | null>(null);
  let breakdown = $state<{ name: string; color: string; consumed: number }[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const isEnergyMeter = (d: DeviceInfo) =>
    d.capabilities.some(
      (c) => c.type === "sensor" && c.sensor_type === "energy_meter",
    );

  let reqToken = 0;

  async function load(p: Period, dev: string | undefined) {
    const token = ++reqToken;
    loading = true;
    error = null;
    try {
      const spanSec = Math.round((p.end.getTime() - p.start.getTime()) / 1000);
      const prev = periodFor(scope, offset - 1);

      const [cur, prevSum] = await Promise.all([
        fetchEnergySummary({
          deviceId: dev,
          start: p.start,
          end: p.end,
          bucketSeconds: p.bucketSeconds,
        }),
        // Previous period total only — one big bucket keeps it cheap.
        fetchEnergySummary({
          deviceId: dev,
          start: prev.start,
          end: prev.end,
          bucketSeconds: spanSec,
        }),
      ]);

      if (token !== reqToken) return;
      summary = cur;
      previousTotal = prevSum.totalConsumed;

      // Per-meter breakdown only makes sense in aggregate mode with ≥2 meters.
      if (!dev) {
        const meters = $sensorsMemory.devices.filter(isEnergyMeter);
        if (meters.length >= 2) {
          const perDevice = await Promise.all(
            meters.map((m) =>
              fetchEnergySummary({
                deviceId: m.device_id,
                start: p.start,
                end: p.end,
                bucketSeconds: spanSec,
              }).then((s) => ({
                name: m.name,
                color: m.color ?? "#3b82f6",
                consumed: s.totalConsumed,
              })),
            ),
          );
          if (token !== reqToken) return;
          breakdown = perDevice
            .filter((d) => d.consumed > 0)
            .sort((a, b) => b.consumed - a.consumed);
        } else {
          breakdown = [];
        }
      } else {
        breakdown = [];
      }
    } catch (err) {
      if (token !== reqToken) return;
      error = err instanceof Error ? err.message : "Échec du chargement";
    } finally {
      if (token === reqToken) loading = false;
    }
  }

  $effect(() => {
    // Re-fetch whenever the period or the target device changes.
    void sensorsMemory.ensureDevices();
    load(period, deviceId);
  });

  // ---- derived figures -----------------------------------------------------
  /** Full, gap-filled timeline so every sub-period is a bar (missing → 0). */
  const slots = $derived.by(() => {
    const p = period;
    const bucketMs = p.bucketSeconds * 1000;
    const byTs = new Map<number, { consumed: number; produced: number }>();
    for (const b of summary?.buckets ?? []) {
      byTs.set(b.timestamp.getTime(), { consumed: b.consumed, produced: b.produced });
    }
    const out: { start: Date; consumed: number; produced: number }[] = [];
    for (let t = p.start.getTime(); t < p.end.getTime() - 1000; t += bucketMs) {
      const hit = byTs.get(t) ?? { consumed: 0, produced: 0 };
      out.push({ start: new Date(t), consumed: hit.consumed, produced: hit.produced });
    }
    return out;
  });

  const hasProduction = $derived((summary?.totalProduced ?? 0) > 0.001);

  const axisMax = $derived.by(() => {
    const m = Math.max(0.001, ...slots.map((s) => Math.max(s.consumed, s.produced)));
    const step = scope === "day" ? 0.5 : 5;
    return Math.ceil(m / step) * step || step;
  });

  const gridSteps = 4;

  const divisor = $derived.by(() => {
    const p = period;
    const bucketMs = p.bucketSeconds * 1000;
    const spanEnd = Math.min(Date.now(), p.end.getTime());
    const elapsed = Math.max(1, Math.ceil((spanEnd - p.start.getTime()) / bucketMs));
    const totalSub = Math.round((p.end.getTime() - p.start.getTime()) / bucketMs);
    return Math.min(elapsed, totalSub || elapsed);
  });

  const average = $derived((summary?.totalConsumed ?? 0) / Math.max(1, divisor));

  const deltaPct = $derived.by(() => {
    if (previousTotal == null || previousTotal <= 0 || !summary) return null;
    return ((summary.totalConsumed - previousTotal) / previousTotal) * 100;
  });

  // ---- tariff / cost -------------------------------------------------------
  const TARIFF_KEY = "homeAutomation:energyTariff";
  let tariff = $state<number>(readTariff());
  function readTariff(): number {
    const raw = typeof localStorage !== "undefined" ? localStorage.getItem(TARIFF_KEY) : null;
    const v = raw ? parseFloat(raw) : NaN;
    return Number.isFinite(v) && v > 0 ? v : 0.2016;
  }
  function editTariff() {
    const input = prompt(
      "Tarif de l'électricité (€ / kWh) :",
      tariff.toString().replace(".", ","),
    );
    if (input == null) return;
    const v = parseFloat(input.replace(",", "."));
    if (Number.isFinite(v) && v > 0) {
      tariff = v;
      try {
        localStorage.setItem(TARIFF_KEY, String(v));
      } catch {
        /* ignore */
      }
    }
  }
  const cost = $derived((summary?.totalConsumed ?? 0) * tariff);

  // ---- hover tooltip -------------------------------------------------------
  let hovered = $state<number | null>(null);

  // ---- formatting ----------------------------------------------------------
  const nf = (v: number, d = 1) =>
    v.toLocaleString("fr-FR", { minimumFractionDigits: d, maximumFractionDigits: d });

  function power(w: number): string {
    return Math.abs(w) >= 1000 ? `${nf(w / 1000, 2)} kW` : `${Math.round(w)} W`;
  }

  function xLabel(i: number): string {
    const d = slots[i]?.start;
    if (!d) return "";
    if (scope === "day") return d.getHours() % 3 === 0 ? `${String(d.getHours()).padStart(2, "0")}h` : "";
    if (scope === "week") return format(d, "EEE", { locale: fr });
    const day = d.getDate();
    return day === 1 || day % 5 === 0 ? String(day) : "";
  }

  function tipDate(i: number): string {
    const d = slots[i]?.start;
    if (!d) return "";
    if (scope === "day")
      return `${String(d.getHours()).padStart(2, "0")}h – ${String((d.getHours() + 1) % 24).padStart(2, "0")}h`;
    return format(d, "EEEE d MMMM", { locale: fr });
  }
</script>

<div class="energy-dashboard">
  <!-- Topbar -->
  <div class="topbar">
    <div class="segmented" role="group" aria-label="Période">
      <button class="seg" class:active={scope === "day"} onclick={() => setScope("day")}>Jour</button>
      <button class="seg" class:active={scope === "week"} onclick={() => setScope("week")}>Semaine</button>
      <button class="seg" class:active={scope === "month"} onclick={() => setScope("month")}>Mois</button>
    </div>
    <div class="period-nav">
      <button class="nav-btn" aria-label="Période précédente" onclick={() => (offset -= 1)}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg>
      </button>
      <span class="period-label">{period.label}</span>
      <button class="nav-btn" aria-label="Période suivante" disabled={atCurrent} onclick={() => (offset += 1)}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
      </button>
    </div>
  </div>

  {#if error}
    <div class="panel state-panel">
      <p class="state-msg">{error}</p>
      <button class="retry" onclick={() => load(period, deviceId)}>Réessayer</button>
    </div>
  {:else}
    <!-- Tiles -->
    <div class="tiles">
      <div class="tile hero">
        <span class="label"><span class="dot consumed"></span>Consommation {scope === "day" ? "du jour" : scope === "week" ? "de la semaine" : "du mois"}</span>
        <span class="big">{nf(summary?.totalConsumed ?? 0)}<span class="u">kWh</span></span>
        {#if deltaPct != null}
          <span class="delta" class:up={deltaPct > 0} class:down={deltaPct <= 0}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round">
              {#if deltaPct > 0}<polyline points="6 15 12 9 18 15" />{:else}<polyline points="6 9 12 15 18 9" />{/if}
            </svg>
            {nf(Math.abs(deltaPct), 0)} %<span class="cmp">vs période précédente</span>
          </span>
        {/if}
      </div>

      <div class="tile">
        <span class="label">Moyenne / {period.subLabel}</span>
        <span class="val">{nf(average)}<span class="unit">kWh</span></span>
        <span class="tile-sub">sur {divisor} {period.subLabel === "heure" ? "h" : "jours"}</span>
      </div>

      {#if hasProduction}
        <div class="tile">
          <span class="label"><span class="dot produced"></span>Production</span>
          <span class="val">{nf(summary?.totalProduced ?? 0)}<span class="unit">kWh</span></span>
          {#if (summary?.totalConsumed ?? 0) > 0}
            <span class="tile-sub prod">{nf(((summary?.totalProduced ?? 0) / (summary?.totalConsumed ?? 1)) * 100, 0)} % de la conso</span>
          {/if}
        </div>
      {/if}

      <div class="tile">
        <span class="label">Pic de puissance</span>
        <span class="val">{power(summary?.peakPower ?? 0)}</span>
        <span class="tile-sub">
          {#if summary?.peakPowerTimestamp}
            {format(summary.peakPowerTimestamp, scope === "day" ? "HH'h'mm" : "d MMM, HH'h'mm", { locale: fr })}
          {:else}—{/if}
        </span>
      </div>

      <button class="tile tile-btn" onclick={editTariff} title="Cliquer pour modifier le tarif">
        <span class="label">Coût estimé</span>
        <span class="val">≈ {nf(cost, cost >= 100 ? 0 : 2)}<span class="unit">€</span></span>
        <span class="tile-sub">tarif {nf(tariff, 2).replace(".", ",")} €/kWh · modifier</span>
      </button>
    </div>

    <!-- Chart -->
    <div class="panel">
      <div class="panel-head">
        <h2 class="panel-title">Consommation par {period.subLabel}</h2>
        {#if hasProduction}
          <div class="legend">
            <span><span class="sw consumed"></span>Consommée</span>
            <span><span class="sw produced"></span>Produite</span>
          </div>
        {/if}
      </div>
      <div class="chart-wrap" class:loading>
        <div class="chart">
          <div class="gridlines">
            {#each Array(gridSteps + 1) as _, g}
              <div class="gl" style="bottom:{(g / gridSteps) * 100}%">
                <span class="yl">{nf((axisMax * g) / gridSteps, scope === "day" ? 1 : 0)}</span>
              </div>
            {/each}
          </div>

          {#if hovered != null && slots[hovered]}
            <div class="tip" style="left:{((hovered + 0.5) / slots.length) * 100}%">
              <div class="t-date">{tipDate(hovered)}</div>
              <div class="t-row"><span class="sw consumed"></span>Consommée <b>{nf(slots[hovered].consumed, 2)} kWh</b></div>
              {#if hasProduction}
                <div class="t-row"><span class="sw produced"></span>Produite <b>{nf(slots[hovered].produced, 2)} kWh</b></div>
              {/if}
            </div>
          {/if}

          {#each slots as s, i}
            <div
              class="col"
              class:hot={hovered === i}
              role="presentation"
              onmouseenter={() => (hovered = i)}
              onmouseleave={() => (hovered = null)}
            >
              <div class="bar consumed" style="height:{(s.consumed / axisMax) * 100}%"></div>
              {#if hasProduction}
                <div class="bar produced" style="height:{(s.produced / axisMax) * 100}%"></div>
              {/if}
            </div>
          {/each}
        </div>
        <div class="xlabels">
          {#each slots as _, i}
            <div class="xl">{xLabel(i)}</div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Breakdown -->
    {#if breakdown.length >= 2}
      {@const bdTotal = breakdown.reduce((a, b) => a + b.consumed, 0)}
      <div class="panel">
        <div class="panel-head"><h2 class="panel-title">Répartition par appareil</h2></div>
        <div class="breakdown-list">
          {#each breakdown as d}
            {@const pct = bdTotal > 0 ? (d.consumed / bdTotal) * 100 : 0}
            <div class="bd-row">
              <div class="bd-top">
                <span class="sw" style="background:{d.color}"></span>
                <span class="nm">{d.name}</span>
                <span class="kwh">{nf(d.consumed)} kWh</span>
                <span class="pct">{nf(pct, 0)} %</span>
              </div>
              <div class="bd-bar"><div class="bd-fill" style="width:{pct}%;background:{d.color}"></div></div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .energy-dashboard {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Topbar */
  .topbar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .segmented {
    display: inline-flex;
    background: #f2f4f7;
    border-radius: 9px;
    padding: 3px;
    gap: 2px;
  }
  .seg {
    padding: 0.38rem 0.85rem;
    border: none;
    background: transparent;
    border-radius: 7px;
    font-size: 0.8125rem;
    font-weight: 600;
    color: #667085;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }
  .seg:hover {
    color: #101828;
  }
  .seg.active {
    background: #fff;
    color: #101828;
    box-shadow: 0 1px 2px rgba(16, 24, 40, 0.12);
  }
  .period-nav {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    margin-left: auto;
    background: #fff;
    border: 1px solid var(--color-card-border);
    border-radius: 9px;
    padding: 3px;
    box-shadow: var(--shadow-sm);
  }
  .nav-btn {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    border-radius: 7px;
    cursor: pointer;
    color: #667085;
    transition:
      background 0.12s,
      color 0.12s;
  }
  .nav-btn:hover:not(:disabled) {
    background: #f2f4f7;
    color: #101828;
  }
  .nav-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .period-label {
    min-width: 128px;
    text-align: center;
    font-size: 0.875rem;
    font-weight: 600;
    color: #101828;
    font-variant-numeric: tabular-nums;
    text-transform: capitalize;
  }

  /* Tiles */
  .tiles {
    display: grid;
    grid-template-columns: 1.5fr repeat(3, 1fr);
    gap: 12px;
  }
  .tile {
    background: var(--color-card-bg);
    border-radius: 14px;
    box-shadow: var(--shadow-sm);
    padding: 1.05rem 1.15rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
    text-align: left;
  }
  .tile-btn {
    border: none;
    font-family: inherit;
    cursor: pointer;
    transition: box-shadow 0.15s;
  }
  .tile-btn:hover {
    box-shadow: var(--shadow-md);
  }
  .label {
    font-size: 0.72rem;
    font-weight: 600;
    color: #667085;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .dot.consumed,
  .sw.consumed {
    background: #3b82f6;
  }
  .dot.produced,
  .sw.produced {
    background: #16a34a;
  }
  .val {
    font-size: 1.05rem;
    font-weight: 700;
    color: #101828;
    font-variant-numeric: tabular-nums;
    display: flex;
    align-items: baseline;
    gap: 0.3rem;
  }
  .unit {
    font-size: 0.8rem;
    font-weight: 600;
    color: #98a2b3;
  }
  .tile-sub {
    font-size: 0.75rem;
    color: #98a2b3;
  }
  .tile-sub.prod {
    color: #16a34a;
    font-weight: 600;
  }
  .tile.hero {
    padding: 1.2rem 1.3rem;
  }
  .hero .big {
    font-size: 2.6rem;
    line-height: 1;
    font-weight: 800;
    letter-spacing: -0.025em;
    font-variant-numeric: tabular-nums;
    color: #101828;
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }
  .hero .big .u {
    font-size: 1rem;
    font-weight: 600;
    color: #98a2b3;
  }
  .delta {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.78rem;
    font-weight: 700;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    width: fit-content;
  }
  .delta.up {
    background: #fef2f2;
    color: #dc2626;
  }
  .delta.down {
    background: #ecfdf3;
    color: #16a34a;
  }
  .delta .cmp {
    color: #98a2b3;
    font-weight: 500;
  }

  /* Panel */
  .panel {
    background: var(--color-card-bg);
    border-radius: 14px;
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  .panel-head {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.9rem 1.15rem;
    border-bottom: 1px solid #f1f3f5;
  }
  .panel-title {
    font-size: 0.95rem;
    font-weight: 700;
    color: #101828;
    margin: 0;
  }
  .legend {
    display: inline-flex;
    gap: 0.9rem;
    margin-left: auto;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
    color: #667085;
    font-weight: 500;
  }
  .sw {
    width: 11px;
    height: 11px;
    border-radius: 3px;
  }

  /* Chart */
  .chart-wrap {
    padding: 1.1rem 1.15rem 1.3rem;
    transition: opacity 0.15s;
  }
  .chart-wrap.loading {
    opacity: 0.45;
  }
  .chart {
    position: relative;
    height: 240px;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding-left: 34px;
    border-bottom: 1px solid var(--color-card-border);
  }
  .gridlines {
    position: absolute;
    left: 34px;
    right: 0;
    top: 0;
    bottom: 0;
    pointer-events: none;
  }
  .gl {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px dashed #eef0f3;
  }
  .yl {
    position: absolute;
    left: -34px;
    top: -8px;
    font-size: 0.68rem;
    color: #98a2b3;
    font-variant-numeric: tabular-nums;
    width: 30px;
    text-align: right;
  }
  .col {
    position: relative;
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 2px;
    cursor: pointer;
    min-width: 0;
  }
  .bar {
    width: 100%;
    max-width: 26px;
    border-radius: 4px 4px 0 0;
    transition: opacity 0.12s;
  }
  .bar.consumed {
    background: #3b82f6;
  }
  .bar.produced {
    background: #16a34a;
  }
  .col.hot .bar {
    opacity: 0.78;
  }
  .xlabels {
    display: flex;
    gap: 2px;
    padding-left: 34px;
    margin-top: 0.5rem;
  }
  .xl {
    flex: 1;
    text-align: center;
    font-size: 0.68rem;
    color: #98a2b3;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
  }
  .tip {
    position: absolute;
    top: -6px;
    transform: translate(-50%, -100%);
    background: #101828;
    color: #fff;
    padding: 0.5rem 0.65rem;
    border-radius: 8px;
    font-size: 0.75rem;
    line-height: 1.5;
    pointer-events: none;
    white-space: nowrap;
    z-index: 5;
    box-shadow: 0 6px 16px rgba(16, 24, 40, 0.22);
  }
  .t-date {
    color: #cbd5e1;
    font-size: 0.68rem;
    margin-bottom: 2px;
    text-transform: capitalize;
  }
  .t-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .t-row .sw {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .tip b {
    font-variant-numeric: tabular-nums;
  }

  /* Breakdown */
  .breakdown-list {
    padding: 1rem 1.15rem 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .bd-row {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .bd-top {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .bd-top .nm {
    color: #475467;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bd-top .kwh {
    margin-left: auto;
    font-weight: 700;
    color: #101828;
    font-variant-numeric: tabular-nums;
  }
  .bd-top .pct {
    color: #98a2b3;
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    width: 40px;
    text-align: right;
  }
  .bd-bar {
    height: 8px;
    border-radius: 999px;
    background: #f0f2f5;
    overflow: hidden;
  }
  .bd-fill {
    height: 100%;
    border-radius: 999px;
  }

  /* States */
  .state-panel {
    padding: 2rem 1.15rem;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.85rem;
  }
  .state-msg {
    margin: 0;
    color: #667085;
    font-size: 0.9rem;
  }
  .retry {
    padding: 0.5rem 1.1rem;
    border: 1px solid var(--color-card-border);
    background: #fff;
    border-radius: 8px;
    font-weight: 600;
    font-size: 0.85rem;
    color: #101828;
    cursor: pointer;
  }
  .retry:hover {
    background: #f7f8fa;
  }

  @media (max-width: 760px) {
    .tiles {
      grid-template-columns: 1fr 1fr;
    }
    .tile.hero {
      grid-column: 1 / -1;
    }
    .legend {
      width: 100%;
      margin: 0.4rem 0 0;
    }
    .panel-head {
      flex-wrap: wrap;
    }
  }
</style>
