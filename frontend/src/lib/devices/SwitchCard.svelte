<script lang="ts">
  import type { SwitchDevice, AutomationRule } from "../api";
  import { executeCommand, fetchAutomationRules } from "../api";
  import { formatDistanceToNow } from "date-fns";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import EditableDeviceName from "./EditableDeviceName.svelte";
  import AutomationRulePanel from "../automation/AutomationRulePanel.svelte";
  import { icons } from "../icons";
  import { slide } from "svelte/transition";
  import { rulesByDevice } from "../stores/automations";
  import { dataCache } from "../stores/dataCache";

  let { device }: { device: SwitchDevice } = $props();

  let isToggling = $state(false);
  let error = $state<string | null>(null);

  // Automation expansion state
  let isExpanded = $state(false);
  let deviceRules = $state<AutomationRule[]>([]);
  let loadingRules = $state(false);
  let rulesError = $state<string | null>(null);

  let displayName = $derived(device.name || device.id);
  let deviceRulesFromStore = $derived($rulesByDevice[device.id] || []);
  let ruleCount = $derived(deviceRulesFromStore.length);
  let shortId = $derived(
    device.id.slice(0, 16) + (device.id.length > 16 ? "..." : "")
  );
  let timeAgo = $derived(
    device.last_seen
      ? formatDistanceToNow(new Date(device.last_seen * 1000), {
          addSuffix: true,
        })
      : "Never"
  );

  // Replace "less than a minute ago" with "now"
  let displayTimeAgo = $derived(
    timeAgo === "less than a minute ago" ? "now" : timeAgo
  );

  async function toggleSwitch() {
    if (isToggling) return;

    isToggling = true;
    error = null;

    const newState = !device.state;
    const previousState = device.state;

    device.state = newState;

    try {
      await executeCommand(device.id, newState);
      dataCache.updateSwitchState(device.id, { state: newState });
    } catch (err) {
      device.state = previousState;
      error = err instanceof Error ? err.message : "Failed to toggle switch";
      console.error("Failed to toggle switch:", err);
    } finally {
      isToggling = false;
    }
  }

  async function toggleExpansion() {
    isExpanded = !isExpanded;

    if (isExpanded && deviceRules.length === 0) {
      await loadAutomationRules();
    }
  }

  async function loadAutomationRules() {
    loadingRules = true;
    rulesError = null;

    try {
      const allRules = await fetchAutomationRules();
      deviceRules = allRules.filter((rule) =>
        rule.actions.some((action) => action.device_id === device.id)
      );
    } catch (err) {
      rulesError = err instanceof Error ? err.message : "Failed to load rules";
    } finally {
      loadingRules = false;
    }
  }

  $effect(() => {
    if (isExpanded) {
      deviceRules = deviceRulesFromStore;
    }
  });
</script>

<div class="switch-card">
  <div class="card-header">
    <div class="device-icon" class:on={device.state}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path d={icons.lightbulb}/>
      </svg>
    </div>
    <div class="device-info">
      <EditableDeviceName deviceId={device.id} name={displayName} />
      <div class="device-id">{shortId}</div>
      <div
        class="last-update"
        title={device.last_seen ? new Date(device.last_seen * 1000).toLocaleString() : "Never"}
      >
        {displayTimeAgo}
      </div>
    </div>
    <div class="badges">
      {#if device.battery_level != null}
        <StatusBadge type="battery" value={device.battery_level} />
      {/if}
      {#if device.link_quality != null}
        <StatusBadge type="signal" value={device.link_quality} />
      {/if}
    </div>
  </div>

  <div class="status-section">
    <div class="status-label">Status</div>
    <div class="status-indicator" class:on={device.state}>
      <div class="status-dot"></div>
      <span>{device.state ? "On" : "Off"}</span>
    </div>
  </div>

  <div class="actions">
    <button
      class="toggle-btn"
      class:on={device.state}
      class:loading={isToggling}
      disabled={isToggling}
      onclick={toggleSwitch}
      type="button"
    >
      {#if isToggling}
        <div class="spinner"></div>
        Switching...
      {:else if device.state}
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d={icons.check}/>
        </svg>
        Turn Off
      {:else}
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d={icons.minus}/>
        </svg>
        Turn On
      {/if}
    </button>
  </div>

  {#if error}
    <div class="error-message">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path d={icons.error}/>
      </svg>
      {error}
    </div>
  {/if}

  <button
    class="automation-toggle-btn"
    onclick={toggleExpansion}
    aria-expanded={isExpanded}
    type="button"
  >
    <div class="automation-toggle-content">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="automation-icon">
        <path d={icons.wrench}/>
      </svg>
      <span>Automation Rules ({ruleCount})</span>
    </div>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="chevron" class:expanded={isExpanded}>
      <path d={icons.chevronDown}/>
    </svg>
  </button>

  {#if isExpanded}
    <div class="automation-section" transition:slide={{ duration: 200 }}>
      <AutomationRulePanel
        deviceId={device.id}
        {deviceRules}
        loading={loadingRules}
        error={rulesError}
      />
    </div>
  {/if}
</div>

<style>
  .switch-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    transition: box-shadow 0.2s ease;
  }

  .switch-card:hover {
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .device-icon {
    width: 48px;
    height: 48px;
    background: linear-gradient(135deg, #6b7280 0%, #4b5563 100%);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    transition: all 0.3s ease;
  }

  .device-icon.on {
    background: linear-gradient(135deg, #fbbf24 0%, #f59e0b 100%);
    box-shadow: 0 4px 12px rgba(251, 191, 36, 0.4);
  }

  .device-icon svg {
    width: 28px;
    height: 28px;
  }

  .device-info {
    flex: 1;
    min-width: 0;
  }

  .device-id {
    font-size: 0.75rem;
    color: #9ca3af;
    font-family: monospace;
  }

  .last-update {
    font-size: 0.75rem;
    color: #6b7280;
    margin-top: 0.125rem;
  }

  .status-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem;
    background: #f9fafb;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .status-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: #6b7280;
  }

  .status-indicator.on {
    color: #16a34a;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #d1d5db;
    transition: all 0.3s ease;
  }

  .status-indicator.on .status-dot {
    background: #16a34a;
    box-shadow: 0 0 8px rgba(22, 163, 74, 0.6);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .toggle-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
    background: white;
    color: #374151;
    font-weight: 600;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .toggle-btn svg {
    width: 18px;
    height: 18px;
  }

  .toggle-btn:hover:not(:disabled) {
    background: #f9fafb;
    border-color: #d1d5db;
    transform: translateY(-2px);
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .toggle-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .toggle-btn.on {
    background: linear-gradient(135deg, #16a34a 0%, #15803d 100%);
    border-color: #16a34a;
    color: white;
  }

  .toggle-btn.on:hover:not(:disabled) {
    background: linear-gradient(135deg, #15803d 0%, #166534 100%);
    border-color: #15803d;
  }

  .toggle-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .toggle-btn.loading {
    position: relative;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #dc2626;
    font-size: 0.875rem;
  }

  .error-message svg {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .automation-toggle-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    margin-top: 0.75rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
  }

  .automation-toggle-btn:hover {
    background: #f3f4f6;
    border-color: #d1d5db;
  }

  .automation-toggle-content {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .automation-icon {
    width: 20px;
    height: 20px;
    color: #6b7280;
  }

  .chevron {
    width: 20px;
    height: 20px;
    color: #9ca3af;
    transition: transform 0.2s ease;
  }

  .chevron.expanded {
    transform: rotate(180deg);
  }

  .automation-section {
    border-top: 1px solid #e5e7eb;
    margin-top: 0.75rem;
  }
</style>
