<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import {
    fetchDeviceState,
    updateDeviceName,
    executeCommand,
    fetchAutomationRules,
  } from "../api";
  import type { SwitchDevice, AutomationRule } from "../api";
  import type { DeviceState } from "../types/devices";
  import { dataCache } from "../stores/dataCache";
  import { automationStore, rulesByDevice } from "../stores/automations";
  import StatusBadge from "../shared/StatusBadge.svelte";
  import AutomationRulePanel from "../automation/AutomationRulePanel.svelte";
  import { icons } from "../icons";

  interface Props {
    params: { id: string };
  }

  let { params }: Props = $props();

  let device = $state<SwitchDevice | null>(null);
  let deviceState = $state<DeviceState | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let editingName = $state(false);
  let editedName = $state("");
  let savingName = $state(false);
  let isToggling = $state(false);
  let toggleError = $state<string | null>(null);

  // Automation rules state
  let deviceRules = $state<AutomationRule[]>([]);
  let loadingRules = $state(false);
  let rulesError = $state<string | null>(null);

  // Get rules from store for this device
  let deviceRulesFromStore = $derived($rulesByDevice[params.id] || []);

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
      // Find switch device info from dataCache
      const switchDevice = $dataCache.switches.byId[params.id];

      if (!switchDevice) {
        error = "Switch not found";
        loading = false;
        return;
      }

      device = switchDevice;
      editedName = switchDevice.name;

      // Fetch device state (battery, link quality)
      const state = await fetchDeviceState(params.id);
      if (state) {
        deviceState = state;
        dataCache.updateDeviceState(params.id, state);
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
      await executeCommand(params.id, newState);
      dataCache.updateSwitchState(params.id, { state: newState });
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
    editedName = device?.name || "";
    editingName = true;
  }

  function cancelEditingName() {
    editingName = false;
    editedName = device?.name || "";
  }

  async function saveName() {
    if (!device || editedName.trim() === device.name) {
      editingName = false;
      return;
    }

    savingName = true;
    try {
      await updateDeviceName(params.id, editedName.trim());
      dataCache.updateDeviceName(params.id, editedName.trim());
      device = { ...device, name: editedName.trim() };
      editingName = false;
    } catch (err) {
      console.error("Failed to save name:", err);
    } finally {
      savingName = false;
    }
  }

  function handleNameKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      void saveName();
    } else if (event.key === "Escape") {
      cancelEditingName();
    }
  }

  function goBack() {
    push("/network");
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

<div class="switch-detail-page">
  <header class="page-header">
    <button class="back-button" onclick={goBack} type="button">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
        width="20"
        height="20"
      >
        <path
          d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"
        />
      </svg>
      Back
    </button>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Loading switch data...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <p>{error}</p>
      <button class="btn btn-primary" onclick={loadData} type="button">
        Retry
      </button>
    </div>
  {:else if device}
    <div class="content">
      <div class="device-header">
        <div class="device-icon" class:on={device.state}>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d={icons.lightbulb} />
          </svg>
        </div>
        <div class="device-info">
          {#if editingName}
            <div class="name-edit">
              <input
                type="text"
                bind:value={editedName}
                onkeydown={handleNameKeydown}
                class="name-input"
                disabled={savingName}
              />
              <button
                class="btn btn-sm btn-primary"
                onclick={saveName}
                disabled={savingName}
                type="button"
              >
                {savingName ? "Saving..." : "Save"}
              </button>
              <button
                class="btn btn-sm btn-secondary"
                onclick={cancelEditingName}
                disabled={savingName}
                type="button"
              >
                Cancel
              </button>
            </div>
          {:else}
            <h1 class="device-name">
              {device.name}
              <button
                class="edit-name-btn"
                onclick={startEditingName}
                title="Edit name"
                type="button"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path
                    d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"
                  />
                </svg>
              </button>
            </h1>
          {/if}
          <p class="device-id">{params.id}</p>
        </div>
      </div>

      <div class="state-section">
        <div class="state-card">
          <div class="state-header">
            <span class="state-label">Current State</span>
            <div class="state-indicator" class:on={device.state}>
              <div class="state-dot"></div>
              <span>{device.state ? "On" : "Off"}</span>
            </div>
          </div>

          <button
            class="toggle-btn"
            class:on={device.state}
            class:loading={isToggling}
            disabled={isToggling}
            onclick={toggleSwitch}
            type="button"
          >
            {#if isToggling}
              <div class="spinner-small"></div>
              Switching...
            {:else if device.state}
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d={icons.check} />
              </svg>
              Turn Off
            {:else}
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d={icons.minus} />
              </svg>
              Turn On
            {/if}
          </button>

          {#if toggleError}
            <div class="toggle-error">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d={icons.error} />
              </svg>
              {toggleError}
            </div>
          {/if}
        </div>
      </div>

      <div class="status-section">
        <h2 class="section-title">Device Status</h2>
        <div class="status-grid">
          {#if device.battery_level != null || deviceState?.battery_level != null}
            <div class="status-item">
              <StatusBadge
                type="battery"
                value={device.battery_level ?? deviceState?.battery_level ?? 0}
              />
            </div>
          {/if}
          {#if device.link_quality != null || deviceState?.link_quality != null}
            <div class="status-item">
              <StatusBadge
                type="signal"
                value={device.link_quality ?? deviceState?.link_quality ?? 0}
              />
            </div>
          {/if}
          <div class="status-item last-seen">
            <span class="status-label">Last seen</span>
            <span class="status-value">
              {formatLastSeen(device.last_seen || deviceState?.last_seen)}
            </span>
          </div>
        </div>
      </div>

      <div class="automation-section">
        <h2 class="section-title">Automation Rules</h2>
        <AutomationRulePanel
          deviceId={params.id}
          {deviceRules}
          loading={loadingRules}
          error={rulesError}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .switch-detail-page {
    min-height: calc(100vh - var(--app-header-height) - var(--app-nav-height));
    background: #f3f4f6;
    padding: 1.5rem;
  }

  .page-header {
    margin-bottom: 1.5rem;
  }

  .back-button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
    cursor: pointer;
    transition: all 0.15s;
  }

  .back-button:hover {
    background: #f9fafb;
    border-color: #9ca3af;
  }

  .loading-state,
  .error-state {
    text-align: center;
    padding: 4rem 2rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(59, 130, 246, 0.2);
    border-top-color: #3b82f6;
    border-radius: 50%;
    margin: 0 auto 1rem;
    animation: spin 0.8s linear infinite;
  }

  .spinner-small {
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

  .content {
    max-width: 900px;
    margin: 0 auto;
  }

  .device-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    margin-bottom: 1.5rem;
  }

  .device-icon {
    width: 64px;
    height: 64px;
    background: linear-gradient(135deg, #6b7280 0%, #4b5563 100%);
    border-radius: 16px;
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
    width: 32px;
    height: 32px;
  }

  .device-info {
    flex: 1;
  }

  .device-name {
    font-size: 1.5rem;
    font-weight: 600;
    color: #111827;
    margin: 0 0 0.25rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .edit-name-btn {
    background: none;
    border: none;
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s;
    padding: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .edit-name-btn:hover {
    opacity: 1;
  }

  .edit-name-btn svg {
    width: 18px;
    height: 18px;
    color: #6b7280;
  }

  .device-id {
    font-size: 0.8125rem;
    color: #6b7280;
    font-family: monospace;
    margin: 0;
  }

  .name-edit {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .name-input {
    font-size: 1.25rem;
    font-weight: 600;
    padding: 0.375rem 0.75rem;
    border: 2px solid #3b82f6;
    border-radius: 6px;
    outline: none;
    min-width: 200px;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-sm {
    padding: 0.375rem 0.75rem;
    font-size: 0.8125rem;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover {
    background: #2563eb;
  }

  .btn-secondary {
    background: #e5e7eb;
    color: #374151;
  }

  .btn-secondary:hover {
    background: #d1d5db;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .state-section {
    margin-bottom: 1.5rem;
  }

  .state-card {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .state-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  .state-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
  }

  .state-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    color: #6b7280;
  }

  .state-indicator.on {
    color: #16a34a;
  }

  .state-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #d1d5db;
    transition: all 0.3s ease;
  }

  .state-indicator.on .state-dot {
    background: #16a34a;
    box-shadow: 0 0 8px rgba(22, 163, 74, 0.6);
  }

  .toggle-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem 1.5rem;
    border: 2px solid #e5e7eb;
    border-radius: 10px;
    background: white;
    color: #374151;
    font-weight: 600;
    font-size: 1rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .toggle-btn svg {
    width: 20px;
    height: 20px;
  }

  .toggle-btn:hover:not(:disabled) {
    background: #f9fafb;
    border-color: #9ca3af;
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

  .toggle-error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    color: #dc2626;
    font-size: 0.875rem;
  }

  .toggle-error svg {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .status-section,
  .automation-section {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
    margin: 0 0 1rem;
  }

  .status-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: center;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-item.last-seen {
    padding: 0.5rem 0.75rem;
    background: #f3f4f6;
    border-radius: 8px;
  }

  .status-item .status-label {
    font-size: 0.8125rem;
    color: #6b7280;
  }

  .status-item .status-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
  }

  @media (max-width: 640px) {
    .switch-detail-page {
      padding: 1rem;
    }

    .device-header {
      flex-direction: column;
      text-align: center;
    }

    .device-name {
      justify-content: center;
    }

    .name-edit {
      flex-wrap: wrap;
      justify-content: center;
    }

    .name-input {
      flex: 1;
      min-width: 150px;
    }

    .state-header {
      flex-direction: column;
      gap: 0.75rem;
      text-align: center;
    }

    .toggle-btn {
      font-size: 0.9375rem;
    }
  }
</style>
