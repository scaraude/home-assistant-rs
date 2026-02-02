<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { fetchAutomationRules } from "../api";
  import type { SwitchDevice, AutomationRule } from "../api";
  import type { DeviceState } from "../types/devices";
  import { deviceStateMemory, switchesMemory } from "../memory";
  import { automationStore, rulesByDevice } from "../stores/automations";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import AutomationRulePanel from "../automation/AutomationRulePanel.svelte";
  import Icon from "../design-system/Icon.svelte";
  import EditableDeviceName from "../devices/EditableDeviceName.svelte";
  import { URI_FRONTEND } from "../shared/constant/URI";

  interface Props {
    params: { id: string };
  }

  let { params }: Props = $props();

  let device = $state<SwitchDevice | null>(null);
  let deviceState = $state<DeviceState | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let editingName = $state(false);
  let isToggling = $state(false);
  let toggleError = $state<string | null>(null);

  // Automation rules state
  let deviceRules = $state<AutomationRule[]>([]);
  let loadingRules = $state(false);
  let rulesError = $state<string | null>(null);

  // Get rules from store for this device
  let deviceRulesFromStore = $derived($rulesByDevice[params.id] || []);

  const deviceName = $derived.by(() => {
    const memoryDevice = $switchesMemory.byId[params.id];
    return memoryDevice?.name ?? device?.name ?? params.id;
  });

  // Sync store rules to local state
  $effect(() => {
    deviceRules = deviceRulesFromStore;
  });

  onMount(async () => {
    await loadData();
    await loadAutomationRules();
  });

  async function loadData() {
    loading = true;
    error = null;

    try {
      await switchesMemory.ensureSwitches();
      const switchDevice = $switchesMemory.byId[params.id];

      if (!switchDevice) {
        error = "Switch not found";
        loading = false;
        return;
      }

      device = switchDevice;

      const state = await deviceStateMemory.ensureDeviceState(params.id);
      if (state) {
        deviceState = state;
      }
    } catch (err) {
      console.error("Failed to load switch data:", err);
      error = err instanceof Error ? err.message : "Failed to load switch data";
    } finally {
      loading = false;
    }
  }

  async function loadAutomationRules() {
    loadingRules = true;
    rulesError = null;

    try {
      const allRules = await fetchAutomationRules();
      automationStore.setRules(allRules);
      deviceRules = allRules.filter((rule) =>
        rule.actions.some((action) => action.device_id === params.id),
      );
    } catch (err) {
      rulesError = err instanceof Error ? err.message : "Failed to load rules";
    } finally {
      loadingRules = false;
    }
  }

  async function toggleSwitch() {
    if (isToggling || !device) return;

    isToggling = true;
    toggleError = null;

    const newState = !device.state;
    const previousState = device.state;

    // Optimistic update
    device = { ...device, state: newState };

    try {
      await switchesMemory.sendSwitchCommand(params.id, newState);
      switchesMemory.updateSwitchState(params.id, { state: newState });
    } catch (err) {
      // Revert on error
      device = { ...device, state: previousState };
      toggleError =
        err instanceof Error ? err.message : "Failed to toggle switch";
      console.error("Failed to toggle switch:", err);
    } finally {
      isToggling = false;
    }
  }

  function startEditingName() {
    editingName = true;
  }

  function handleNameSaved() {
    editingName = false;
    const memoryDevice = $switchesMemory.byId[params.id];
    if (memoryDevice) {
      device = memoryDevice;
    }
  }

  function goBack() {
    push(URI_FRONTEND.FLOORPLAN);
  }

  function formatLastSeen(date: Date | null | undefined): string {
    if (!date) return "Unknown";
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (seconds < 60) return "Just now";
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return date.toLocaleDateString();
  }
</script>

<div class="detail-page">
  <!-- Ambient background -->
  <div class="ambient-bg"></div>
  <div class="ambient-glow" class:active={device?.state}></div>

  <header class="page-header">
    <button class="back-btn" onclick={goBack} type="button">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <path d="M19 12H5M12 19l-7-7 7-7" />
      </svg>
      <span>Back</span>
    </button>

    <div class="header-badges">
      {#if device?.battery_level != null || deviceState?.battery_level != null}
        <StatusBadge
          type="battery"
          value={device?.battery_level ?? deviceState?.battery_level ?? 0}
        />
      {/if}
      {#if device?.link_quality != null || deviceState?.link_quality != null}
        <StatusBadge
          type="signal"
          value={device?.link_quality ?? deviceState?.link_quality ?? 0}
        />
      {/if}
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="loader">
        <div class="loader-ring"></div>
        <div class="loader-ring"></div>
        <div class="loader-ring"></div>
      </div>
      <p>Loading switch data...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <div class="error-icon">
        <Icon name="error" size={48} />
      </div>
      <p>{error}</p>
      <button class="btn-primary" onclick={loadData} type="button">
        Retry
      </button>
    </div>
  {:else if device}
    <div class="content">
      <!-- Hero Section with Giant Toggle -->
      <section class="hero-section">
        <div class="device-identity">
          <div class="device-icon-wrapper">
            <div class="device-icon" class:on={device.state}>
              <Icon name="lightbulb" size={32} />
            </div>
            {#if device.state}
              <div class="pulse-ring"></div>
            {/if}
          </div>

          <div class="device-meta">
            <h1 class="device-name">
              <EditableDeviceName
                deviceId={params.id}
                name={deviceName}
                isEdit={editingName}
                onSaved={handleNameSaved}
                class="device-name-text"
              />
              {#if !editingName}
                <button
                  class="edit-btn"
                  onclick={startEditingName}
                  title="Edit name"
                  type="button"
                >
                  <svg
                    viewBox="0 0 24 24"
                    fill="currentColor"
                    width="16"
                    height="16"
                  >
                    <path
                      d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"
                    />
                  </svg>
                </button>
              {/if}
            </h1>
            <p class="device-id">{params.id}</p>
            <p class="last-seen">
              Last seen: {formatLastSeen(
                device.last_seen || deviceState?.last_seen,
              )}
            </p>
          </div>
        </div>

        <!-- Giant Power Control -->
        <div class="power-control">
          <button
            class="power-button"
            class:on={device.state}
            class:toggling={isToggling}
            disabled={isToggling}
            onclick={toggleSwitch}
            type="button"
            aria-label={device.state ? "Turn off" : "Turn on"}
          >
            <div class="power-button-inner">
              <div class="power-icon">
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                >
                  <path d="M12 2v10M18.4 6.6a9 9 0 1 1-12.8 0" />
                </svg>
              </div>
              {#if isToggling}
                <div class="power-spinner"></div>
              {/if}
            </div>
            <div class="power-glow"></div>
          </button>

          <div class="power-status">
            <div class="status-indicator" class:on={device.state}>
              <div class="status-dot"></div>
              <span class="status-text">{device.state ? "ON" : "OFF"}</span>
            </div>
            <p class="status-hint">
              {isToggling
                ? "Switching..."
                : `Tap to turn ${device.state ? "off" : "on"}`}
            </p>
          </div>
        </div>

        {#if toggleError}
          <div class="error-banner">
            <Icon name="error" size={18} />
            <span>{toggleError}</span>
          </div>
        {/if}
      </section>

      <!-- Automation Section -->
      <section class="automation-section">
        <div class="section-header">
          <div class="section-title-group">
            <Icon name="wrench" size={20} />
            <h2>Automation Rules</h2>
          </div>
          <span class="rule-count">{deviceRules.length} rules</span>
        </div>

        <AutomationRulePanel
          deviceId={params.id}
          {deviceRules}
          loading={loadingRules}
          error={rulesError}
        />
      </section>
    </div>
  {/if}
</div>

<style>
  /* === Design Tokens === */
  .detail-page {
    --accent-primary: #10b981;
    --accent-on: #22c55e;
    --accent-off: #64748b;
    --accent-danger: #ef4444;
    --surface-elevated: rgba(255, 255, 255, 0.98);
    --text-primary: #0f172a;
    --text-secondary: #475569;
    --text-muted: #94a3b8;
    --border-subtle: rgba(0, 0, 0, 0.06);
  }

  .detail-page {
    position: relative;
    min-height: 100vh;
    background: linear-gradient(180deg, #f8fafc 0%, #f1f5f9 50%, #e2e8f0 100%);
    padding: var(--space-4);
    overflow-x: hidden;
  }

  /* Ambient backgrounds */
  .ambient-bg {
    position: fixed;
    inset: 0;
    background: radial-gradient(
        ellipse at 50% 0%,
        rgba(16, 185, 129, 0.03) 0%,
        transparent 50%
      ),
      radial-gradient(
        ellipse at 80% 80%,
        rgba(59, 130, 246, 0.03) 0%,
        transparent 50%
      );
    pointer-events: none;
    z-index: 0;
  }

  .ambient-glow {
    position: fixed;
    top: -30%;
    left: 50%;
    transform: translateX(-50%);
    width: 120vw;
    height: 60vh;
    background: radial-gradient(
      ellipse,
      rgba(34, 197, 94, 0) 0%,
      transparent 70%
    );
    pointer-events: none;
    z-index: 0;
    transition: all 0.6s ease;
  }

  .ambient-glow.active {
    background: radial-gradient(
      ellipse,
      rgba(34, 197, 94, 0.12) 0%,
      transparent 70%
    );
  }

  /* === Header === */
  .page-header {
    position: relative;
    z-index: 10;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-6);
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  }

  .back-btn:hover {
    background: white;
    color: var(--text-primary);
    transform: translateX(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  }

  .back-btn svg {
    width: 18px;
    height: 18px;
  }

  .header-badges {
    display: flex;
    gap: var(--space-2);
  }

  /* === Loading / Error States === */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    gap: var(--space-4);
  }

  .loader {
    position: relative;
    width: 60px;
    height: 60px;
  }

  .loader-ring {
    position: absolute;
    inset: 0;
    border: 3px solid transparent;
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1.2s ease-in-out infinite;
  }

  .loader-ring:nth-child(2) {
    inset: 8px;
    border-top-color: #3b82f6;
    animation-delay: 0.15s;
  }

  .loader-ring:nth-child(3) {
    inset: 16px;
    border-top-color: #f59e0b;
    animation-delay: 0.3s;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading-state p {
    font-size: 0.9375rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    gap: var(--space-4);
    text-align: center;
  }

  .error-icon {
    width: 80px;
    height: 80px;
    background: linear-gradient(135deg, #fef2f2, #fee2e2);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-danger);
  }

  .error-state p {
    font-size: 1rem;
    color: var(--text-secondary);
  }

  /* === Content === */
  .content {
    position: relative;
    z-index: 5;
    max-width: 600px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* === Hero Section === */
  .hero-section {
    background: var(--surface-elevated);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 4px 24px rgba(0, 0, 0, 0.04);
    border: 1px solid var(--border-subtle);
  }

  .device-identity {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-8);
    padding-bottom: var(--space-5);
    border-bottom: 1px solid var(--border-subtle);
  }

  .device-icon-wrapper {
    position: relative;
    flex-shrink: 0;
  }

  .device-icon {
    width: 64px;
    height: 64px;
    background: linear-gradient(135deg, #64748b, #475569);
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    transition: all 0.4s ease;
  }

  .device-icon.on {
    background: linear-gradient(135deg, #fbbf24, #f59e0b);
    box-shadow: 0 4px 20px rgba(251, 191, 36, 0.4);
  }

  .pulse-ring {
    position: absolute;
    inset: -4px;
    border: 2px solid rgba(251, 191, 36, 0.4);
    border-radius: 20px;
    animation: pulse-ring 2s ease-out infinite;
  }

  @keyframes pulse-ring {
    0% {
      transform: scale(1);
      opacity: 1;
    }
    100% {
      transform: scale(1.2);
      opacity: 0;
    }
  }

  .device-meta {
    flex: 1;
    min-width: 0;
  }

  .device-name {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 var(--space-1);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    letter-spacing: -0.02em;
  }

  .edit-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: var(--space-1);
    opacity: 0.4;
    transition: opacity 0.2s;
    display: flex;
    align-items: center;
    color: var(--text-secondary);
  }

  .edit-btn:hover {
    opacity: 1;
  }

  .device-id {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0 0 var(--space-1);
  }

  .last-seen {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin: 0;
  }

  :global(.device-name-text) {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  /* === Power Control === */
  .power-control {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-5);
  }

  .power-button {
    position: relative;
    width: 140px;
    height: 140px;
    border-radius: 50%;
    border: none;
    cursor: pointer;
    transition: all 0.3s ease;
    background: none;
    padding: 0;
  }

  .power-button:disabled {
    cursor: not-allowed;
  }

  .power-button-inner {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: linear-gradient(145deg, #e2e8f0, #cbd5e1);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow:
      0 4px 20px rgba(0, 0, 0, 0.1),
      inset 0 -4px 10px rgba(0, 0, 0, 0.05),
      inset 0 4px 10px rgba(255, 255, 255, 0.8);
    transition: all 0.3s ease;
  }

  .power-button.on .power-button-inner {
    background: linear-gradient(145deg, #22c55e, #16a34a);
    box-shadow:
      0 4px 30px rgba(34, 197, 94, 0.4),
      inset 0 -4px 10px rgba(0, 0, 0, 0.1),
      inset 0 4px 10px rgba(255, 255, 255, 0.2);
  }

  .power-button:hover:not(:disabled) .power-button-inner {
    transform: scale(1.02);
  }

  .power-button:active:not(:disabled) .power-button-inner {
    transform: scale(0.98);
  }

  .power-icon {
    width: 48px;
    height: 48px;
    color: #64748b;
    transition: color 0.3s ease;
  }

  .power-button.on .power-icon {
    color: white;
  }

  .power-icon svg {
    width: 100%;
    height: 100%;
  }

  .power-glow {
    position: absolute;
    inset: -20px;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      rgba(34, 197, 94, 0) 30%,
      transparent 70%
    );
    pointer-events: none;
    transition: all 0.4s ease;
    z-index: -1;
  }

  .power-button.on .power-glow {
    background: radial-gradient(
      circle,
      rgba(34, 197, 94, 0.3) 30%,
      transparent 70%
    );
    animation: glow-pulse 2s ease-in-out infinite;
  }

  @keyframes glow-pulse {
    0%,
    100% {
      opacity: 0.7;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.05);
    }
  }

  .power-spinner {
    position: absolute;
    inset: 10px;
    border: 3px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .power-status {
    text-align: center;
  }

  .status-indicator {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: #f1f5f9;
    border-radius: var(--radius-pill);
    margin-bottom: var(--space-2);
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #94a3b8;
    transition: all 0.3s ease;
  }

  .status-indicator.on .status-dot {
    background: #22c55e;
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.6);
  }

  .status-text {
    font-size: 0.875rem;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.05em;
  }

  .status-indicator.on .status-text {
    color: #16a34a;
  }

  .status-hint {
    font-size: 0.8125rem;
    color: var(--text-muted);
    margin: 0;
  }

  .error-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-4);
    padding: var(--space-3);
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: var(--radius-md);
    color: #dc2626;
    font-size: 0.875rem;
  }

  /* === Automation Section === */
  .automation-section {
    background: var(--surface-elevated);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 4px 24px rgba(0, 0, 0, 0.04);
    border: 1px solid var(--border-subtle);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-4);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
  }

  .section-title-group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
  }

  .section-title-group h2 {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .rule-count {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-muted);
    background: #f1f5f9;
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-pill);
  }

  /* === Buttons === */
  .btn-primary {
    padding: var(--space-3) var(--space-5);
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: 0.9375rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-primary:hover {
    background: #059669;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
  }

  /* === Responsive === */
  @media (max-width: 640px) {
    .detail-page {
      padding: var(--space-3);
    }

    .hero-section {
      padding: var(--space-4);
    }

    .device-identity {
      flex-direction: column;
      text-align: center;
    }

    .device-name {
      justify-content: center;
      font-size: 1.25rem;
    }

    .power-button {
      width: 120px;
      height: 120px;
    }

    .power-icon {
      width: 40px;
      height: 40px;
    }
  }
</style>
