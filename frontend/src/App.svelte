<script lang="ts">
  import { onMount } from "svelte";
  import Router, { location } from "svelte-spa-router";
  import SensorsView from "./routes/SensorsView.svelte";
  import LogsView from "./routes/LogsView.svelte";
  import CommanderView from "./routes/CommanderView.svelte";
  import FloorPlanView from "./routes/FloorPlanView.svelte";
  import ConsommationsView from "./routes/ConsommationsView.svelte";
  import SensorDetailPage from "./lib/device-detail/SensorDetailPage.svelte";
  import { setZigbeePermitJoin } from "./lib/api";
  import {
    deviceStateMemory,
    sensorsMemory,
    switchesMemory,
  } from "./lib/memory";
  import { eventStream, type SystemEvent } from "./lib/websocket";
  import SwitchDetailPage from "./lib/device-detail/SwitchDetailPage.svelte";
  import EnergyDetailPage from "./lib/device-detail/EnergyDetailPage.svelte";
  import PresenceDetailPage from "./lib/device-detail/PresenceDetailPage.svelte";
  import { URI_FRONTEND } from "./lib/shared/constant/URI";

  // Route definitions
  const routes = {
    "/": FloorPlanView,
    [URI_FRONTEND.SENSORS]: SensorsView,
    [URI_FRONTEND.SWITCH_DETAIL(":id")]: SwitchDetailPage,
    [URI_FRONTEND.ENERGY_DETAIL(":id")]: EnergyDetailPage,
    [URI_FRONTEND.PRESENCE_DETAIL(":id")]: PresenceDetailPage,
    [URI_FRONTEND.CONSOMMATIONS]: ConsommationsView,
    [URI_FRONTEND.COMMANDER]: CommanderView,
    [URI_FRONTEND.FLOORPLAN]: FloorPlanView,
    [URI_FRONTEND.LOGS]: LogsView,
    [URI_FRONTEND.SENSOR_DETAIL(":id")]: SensorDetailPage,
  };

  const PERMIT_JOIN_SECONDS = 180;
  let initializing = $state(true);
  let initError = $state<string | null>(null);
  let initialLoadInFlight = $state(false);
  let permitJoinSecondsRemaining = $state<number | null>(null);
  let permitJoinInFlight = $state(false);
  let permitJoinTimer: ReturnType<typeof setInterval> | null = null;

  async function loadInitialData() {
    if (initialLoadInFlight) {
      return;
    }

    initialLoadInFlight = true;
    initializing = true;
    initError = null;

    try {
      await Promise.all([
        sensorsMemory.ensureDevices(),
        switchesMemory.ensureSwitches(),
      ]);
    } catch (error) {
      console.error("Failed to load initial data:", error);
      initError =
        error instanceof Error ? error.message : "Failed to load initial data";
    } finally {
      initializing = false;
      initialLoadInFlight = false;
    }
  }

  function normalizeSwitchState(state: boolean | string): boolean {
    if (typeof state === "string") {
      return state.toUpperCase() === "ON";
    }
    return state;
  }

  function handleEvent(event: SystemEvent) {
    switch (event.event) {
      case "sensor_reading":
        sensorsMemory.mergeIncomingReading(event.reading);
        break;
      case "switch_state":
        switchesMemory.updateSwitchState(event.device_id, {
          state: normalizeSwitchState(event.state),
          last_seen: event.timestamp,
        });
        deviceStateMemory.updateDeviceState(event.device_id, {
          last_seen: event.timestamp,
        });
        break;
      case "device_state":
        deviceStateMemory.updateDeviceState(event.device_id, {
          battery_level: event.battery,
          link_quality: event.link_quality,
          turbo_mode: event.turbo_mode,
          last_seen: event.timestamp,
        });
        switchesMemory.applyDeviceState(event.device_id, {
          device_id: event.device_id,
          battery_level: event.battery,
          link_quality: event.link_quality,
          turbo_mode: event.turbo_mode,
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

  function formatCountdown(totalSeconds: number): string {
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  function stopPermitJoinCountdown() {
    if (permitJoinTimer) {
      clearInterval(permitJoinTimer);
      permitJoinTimer = null;
    }
    permitJoinSecondsRemaining = null;
  }

  function startPermitJoinCountdown(seconds: number) {
    stopPermitJoinCountdown();
    permitJoinSecondsRemaining = seconds;
    permitJoinTimer = setInterval(() => {
      if (permitJoinSecondsRemaining === null) {
        stopPermitJoinCountdown();
        return;
      }
      if (permitJoinSecondsRemaining <= 1) {
        stopPermitJoinCountdown();
        return;
      }
      permitJoinSecondsRemaining -= 1;
    }, 1000);
  }

  async function togglePermitJoin() {
    if (permitJoinInFlight) {
      return;
    }

    if (permitJoinSecondsRemaining !== null) {
      return;
    }

    permitJoinInFlight = true;
    try {
      await setZigbeePermitJoin(true, PERMIT_JOIN_SECONDS);
      startPermitJoinCountdown(PERMIT_JOIN_SECONDS);
    } catch (error) {
      console.error("Failed to update Zigbee permit join:", error);
    } finally {
      permitJoinInFlight = false;
    }
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
      stopPermitJoinCountdown();
    };
  });

  // Track current route for active state
  let currentPath = $derived($location);
  let isOnFloorPlan = $derived(
    currentPath === "/" || currentPath === URI_FRONTEND.FLOORPLAN,
  );
  let isOnSensors = $derived(currentPath === URI_FRONTEND.SENSORS);
  let isOnConsommations = $derived(currentPath === URI_FRONTEND.CONSOMMATIONS);
  let isOnCommander = $derived(currentPath === URI_FRONTEND.COMMANDER);
  let isOnLogs = $derived(currentPath === URI_FRONTEND.LOGS);
  let permitJoinActive = $derived(permitJoinSecondsRemaining !== null);
  let permitJoinLabel = $derived(
    permitJoinSecondsRemaining === null
      ? "Open network"
      : `Network open ${formatCountdown(permitJoinSecondsRemaining)}`,
  );
</script>

<main>
  <header>
    <div class="header-content">
      <button
        onclick={() => (window.location.href = "/#/")}
        class="home-button"
        aria-label="Go to Home"
      >
        <div class="logo">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z" />
          </svg>
        </div>
        <h1>Home Automation</h1>
      </button>

      <nav class="nav-buttons">
        <a
          href={"/#" + URI_FRONTEND.FLOORPLAN}
          class="nav-button"
          class:active={isOnFloorPlan}
        >
          Floor Plan
        </a>
        <a
          href={"/#" + URI_FRONTEND.SENSORS}
          class="nav-button"
          class:active={isOnSensors}
        >
          Sensors
        </a>
        <a
          href={"/#" + URI_FRONTEND.CONSOMMATIONS}
          class="nav-button"
          class:active={isOnConsommations}
        >
          Energy
        </a>
        <a
          href={"/#" + URI_FRONTEND.COMMANDER}
          class="nav-button"
          class:active={isOnCommander}
        >
          Commander
        </a>
        <a
          href={"/#" + URI_FRONTEND.LOGS}
          class="nav-button"
          class:active={isOnLogs}
        >
          System Logs
        </a>
        <button
          type="button"
          class="permit-join-button"
          class:active={permitJoinActive}
          class:loading={permitJoinInFlight}
          onclick={togglePermitJoin}
          aria-pressed={permitJoinActive}
          aria-label="Toggle Zigbee network permit join"
          title={permitJoinLabel}
        >
          <span class="permit-join-icon" aria-hidden="true">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path
                d="M12 3a9 9 0 0 1 9 9h-2a7 7 0 0 0-14 0H3a9 9 0 0 1 9-9zm0 4a5 5 0 0 1 5 5h-2a3 3 0 0 0-6 0H7a5 5 0 0 1 5-5zm0 4a1 1 0 0 1 1 1v9h-2v-9a1 1 0 0 1 1-1z"
              />
            </svg>
          </span>
          <span class="permit-join-label">{permitJoinLabel}</span>
        </button>
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
  :global(:root) {
    --app-header-height: 88px;
    --app-nav-height: 0px;
  }

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
    align-items: center;
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

  .permit-join-button {
    height: 2.25rem;
    width: 2.25rem;
    padding: 0;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    background: white;
    color: #6b7280;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    cursor: pointer;
    overflow: hidden;
    transition:
      width 0.25s ease,
      background 0.2s ease,
      border-color 0.2s ease,
      color 0.2s ease;
  }

  .permit-join-button:hover:not(.loading):not(.active) {
    background: #f9fafb;
    color: #111827;
    border-color: #d1d5db;
  }

  .permit-join-button.active {
    width: 11.5rem;
    padding: 0 0.75rem;
    justify-content: flex-start;
    background: #fef8e3;
    border-color: #f3d59a;
    color: #b0792da0;
    cursor: default;
  }

  .permit-join-button.loading {
    opacity: 0.7;
    cursor: wait;
  }

  .permit-join-icon {
    width: 1.2rem;
    height: 1.2rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .permit-join-icon svg {
    width: 100%;
    height: 100%;
  }

  .permit-join-label {
    font-size: 0.8125rem;
    font-weight: 600;
    white-space: nowrap;
    opacity: 0;
    max-width: 0;
    transition:
      opacity 0.2s ease,
      max-width 0.25s ease;
  }

  .permit-join-button.active .permit-join-label {
    opacity: 1;
    max-width: 10rem;
  }

  .home-button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: none;
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
    max-width: none;
    margin: 0 auto;
    /* padding: 2rem 1rem; */
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
    :global(:root) {
      --app-header-height: 96px;
      --app-nav-height: 76px;
    }

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
      flex-wrap: wrap;
    }

    .nav-button {
      flex: 1;
      padding: 0.75rem;
    }

    .permit-join-button {
      flex: 0 0 auto;
      height: 2.75rem;
      width: 2.75rem;
    }

    .permit-join-button.active {
      width: 9.5rem;
    }

    .container {
      /* padding: var(--container-padding, 1rem); */
      padding-bottom: 5rem;
    }

    .init-error {
      flex-direction: column;
      align-items: flex-start;
    }
  }

  /* Small phones - compact nav */
  @media (max-width: 480px) {
    :global(:root) {
      --app-header-height: 84px;
      --app-nav-height: 64px;
    }

    header {
      padding: 0.75rem 0;
    }

    .header-content {
      padding: 0 var(--container-padding, 0.75rem);
      gap: 0.5rem;
    }

    .logo {
      width: 32px;
      height: 32px;
      border-radius: 6px;
    }

    .logo svg {
      width: 20px;
      height: 20px;
    }

    h1 {
      font-size: 1.125rem;
    }

    .nav-buttons {
      padding: 0.5rem;
      gap: 0.375rem;
    }

    .nav-button {
      padding: 0.625rem 0.5rem;
      font-size: 0.75rem;
      min-width: 0;
    }

    .permit-join-button {
      height: 2.5rem;
      width: 2.5rem;
    }

    .permit-join-button.active {
      width: 8rem;
    }

    .permit-join-label {
      font-size: 0.75rem;
    }

    .container {
      /* padding: var(--container-padding, 0.75rem); */
      padding-bottom: 4.5rem;
    }

    .sync-indicator {
      font-size: 0.75rem;
    }
  }

  /* Very small phones (320px) */
  @media (max-width: 375px) {
    .nav-button {
      padding: 0.5rem 0.25rem;
      font-size: 0.6875rem;
    }

    .permit-join-button.active {
      width: 7rem;
    }
  }
</style>
