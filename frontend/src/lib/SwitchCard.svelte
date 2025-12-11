<script lang="ts">
  import type { SwitchDevice, AutomationRule } from "./api";
  import { executeCommand, updateDeviceName, fetchAutomationRules } from "./api";
  import { formatDistanceToNow } from "date-fns";
  import LinkQualityBadge from "./LinkQualityBadge.svelte";
  import { slide } from "svelte/transition";
  import { rulesByDevice } from "./stores/automations";

  export let device: SwitchDevice;

  let isToggling = false;
  let error: string | null = null;
  let isEditingName = false;
  let editedName = "";

  // Automation expansion state
  let isExpanded = false;
  let deviceRules: AutomationRule[] = [];
  let loadingRules = false;
  let rulesError: string | null = null;

  $: displayName = device.name || device.id;
  $: deviceRulesFromStore = $rulesByDevice[device.id] || [];
  $: ruleCount = deviceRulesFromStore.length;
  $: shortId = device.id.slice(0, 16) + (device.id.length > 16 ? "..." : "");
  $: timeAgo = formatDistanceToNow(new Date(device.last_updated * 1000), {
    addSuffix: true,
  });

  async function toggleSwitch() {
    if (isToggling) return;

    isToggling = true;
    error = null;

    const newState = !device.state;
    const previousState = device.state;

    // Optimistic update
    device.state = newState;

    try {
      await executeCommand(device.id, newState);
    } catch (err) {
      // Revert on error
      device.state = previousState;
      error = err instanceof Error ? err.message : "Failed to toggle switch";
      console.error("Failed to toggle switch:", err);
    } finally {
      isToggling = false;
    }
  }

  function startEditingName() {
    editedName = device.name;
    isEditingName = true;
  }

  async function saveName() {
    const trimmedName = editedName.trim();

    if (!trimmedName) {
      error = "Device name cannot be empty";
      return;
    }

    if (trimmedName === device.name) {
      isEditingName = false;
      return;
    }

    error = null;
    const previousName = device.name;

    try {
      // Optimistic update
      device.name = trimmedName;
      isEditingName = false;

      await updateDeviceName(device.id, trimmedName);
    } catch (err) {
      // Revert on error
      device.name = previousName;
      isEditingName = true;
      error = err instanceof Error ? err.message : "Failed to update device name";
      console.error("Failed to update device name:", err);
    }
  }

  function cancelEdit() {
    isEditingName = false;
    error = null;
  }

  function handleNameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      saveName();
    } else if (e.key === "Escape") {
      cancelEdit();
    }
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
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
      // Filter rules that have actions for this device
      deviceRules = allRules.filter((rule) =>
        rule.actions.some((action) => action.device_id === device.id)
      );
    } catch (err) {
      rulesError = err instanceof Error ? err.message : "Failed to load rules";
    } finally {
      loadingRules = false;
    }
  }

  // Update deviceRules when store changes
  $: if (isExpanded) {
    deviceRules = deviceRulesFromStore;
  }
</script>

<div class="switch-card">
  <div class="card-header">
    <div class="device-icon" class:on={device.state}>
      <!-- Light Bulb Icon -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <path
          d="M9 21c0 .55.45 1 1 1h4c.55 0 1-.45 1-1v-1H9v1zm3-19C8.14 2 5 5.14 5 9c0 2.38 1.19 4.47 3 5.74V17c0 .55.45 1 1 1h6c.55 0 1-.45 1-1v-2.26c1.81-1.27 3-3.36 3-5.74 0-3.86-3.14-7-7-7z"
        />
      </svg>
    </div>
    <div class="device-info">
      {#if isEditingName}
        <input
          type="text"
          class="device-name-input"
          bind:value={editedName}
          on:keydown={handleNameKeydown}
          on:blur={saveName}
          use:focusOnMount
        />
      {:else}
        <button
          class="device-name editable"
          title={device.id}
          on:click={startEditingName}
          type="button"
        >
          {displayName}
        </button>
      {/if}
      <div class="device-id">{shortId}</div>
      <div
        class="last-update"
        title={new Date(device.last_updated * 1000).toLocaleString()}
      >
        {timeAgo}
      </div>
    </div>
    {#if device.link_quality != null}
      <LinkQualityBadge linkQuality={device.link_quality} />
    {/if}
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
      on:click={toggleSwitch}
      type="button"
    >
      {#if isToggling}
        <div class="spinner"></div>
        Switching...
      {:else if device.state}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path
            d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"
          />
        </svg>
        Turn Off
      {:else}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path
            d="M7 11v2h10v-2H7zm5-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z"
          />
        </svg>
        Turn On
      {/if}
    </button>
  </div>

  {#if error}
    <div class="error-message">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <path
          d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
        />
      </svg>
      {error}
    </div>
  {/if}

  <!-- Automation Rules Toggle Button -->
  <button
    class="automation-toggle-btn"
    on:click={toggleExpansion}
    aria-expanded={isExpanded}
    type="button"
  >
    <div class="automation-toggle-content">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
        class="automation-icon"
      >
        <path
          d="M22.7 19l-9.1-9.1c.9-2.3.4-5-1.5-6.9-2-2-5-2.4-7.4-1.3L9 6 6 9 1.6 4.7C.4 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1.1.1-1.4z"
        />
      </svg>
      <span>Automation Rules ({ruleCount})</span>
    </div>
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="currentColor"
      class="chevron"
      class:expanded={isExpanded}
    >
      <path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z" />
    </svg>
  </button>

  <!-- Expandable Automation Section -->
  {#if isExpanded}
    <div class="automation-section" transition:slide={{ duration: 200 }}>
      {#if loadingRules}
        <div class="loading-state">
          <div class="spinner"></div>
          <span>Loading rules...</span>
        </div>
      {:else if rulesError}
        <div class="error-state">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path
              d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
            />
          </svg>
          <span>{rulesError}</span>
          <button class="retry-btn" on:click={loadAutomationRules} type="button">
            Retry
          </button>
        </div>
      {:else}
        <div class="rules-list">
          {#each deviceRules as rule (rule.id)}
            <div class="rule-item" class:disabled={!rule.enabled}>
              <div class="rule-header">
                <div class="rule-info">
                  <h4 class="rule-name">{rule.name}</h4>
                  {#if rule.description}
                    <p class="rule-description">{rule.description}</p>
                  {/if}
                </div>
                <div class="rule-status" class:enabled={rule.enabled}>
                  {rule.enabled ? "Enabled" : "Disabled"}
                </div>
              </div>

              <div class="rule-details">
                <div class="rule-section">
                  <span class="section-label">
                    Conditions ({rule.condition_operator.toUpperCase()}):
                  </span>
                  <ul class="conditions-list">
                    {#each rule.conditions as condition}
                      <li>
                        {condition.field}
                        {condition.operator.replace("_", " ")}
                        {condition.value}
                      </li>
                    {/each}
                  </ul>
                </div>

                {#if rule.last_triggered_at}
                  <div class="rule-meta">
                    Last triggered: {formatDistanceToNow(
                      new Date(rule.last_triggered_at),
                      { addSuffix: true }
                    )}
                    (Triggered {rule.trigger_count} {rule.trigger_count === 1
                      ? "time"
                      : "times"})
                  </div>
                {/if}
              </div>
            </div>
          {/each}

          {#if deviceRules.length === 0}
            <div class="empty-state">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path
                  d="M22.7 19l-9.1-9.1c.9-2.3.4-5-1.5-6.9-2-2-5-2.4-7.4-1.3L9 6 6 9 1.6 4.7C.4 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1.1.1-1.4z"
                />
              </svg>
              <p>No automation rules for this switch yet</p>
              <p class="empty-hint">
                Create a rule to automate this switch based on sensor readings
              </p>
            </div>
          {/if}
        </div>
      {/if}
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

  .device-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .device-name.editable {
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    transition: color 0.2s ease;
  }

  .device-name.editable:hover {
    color: #3b82f6;
  }

  .device-name-input {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
    border: 2px solid #3b82f6;
    border-radius: 4px;
    padding: 0.25rem 0.5rem;
    width: 100%;
    outline: none;
    background: white;
  }

  .device-name-input:focus {
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
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

  /* Automation Toggle Button */
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

  /* Automation Section */
  .automation-section {
    border-top: 1px solid #e5e7eb;
    padding: 1rem;
    background: #fafbfc;
    margin-top: 0.75rem;
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* Rule Item */
  .rule-item {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    padding: 1rem;
    transition: all 0.2s ease;
  }

  .rule-item:hover {
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  }

  .rule-item.disabled {
    opacity: 0.6;
  }

  .rule-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 0.75rem;
    gap: 0.75rem;
  }

  .rule-info {
    flex: 1;
    min-width: 0;
  }

  .rule-name {
    margin: 0;
    font-size: 0.9375rem;
    font-weight: 600;
    color: #111827;
  }

  .rule-description {
    margin: 0.25rem 0 0;
    font-size: 0.8125rem;
    color: #6b7280;
    line-height: 1.4;
  }

  .rule-status {
    padding: 0.25rem 0.625rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    background: #f3f4f6;
    color: #6b7280;
    white-space: nowrap;
  }

  .rule-status.enabled {
    background: #dcfce7;
    color: #16a34a;
  }

  .rule-details {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .rule-section {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .section-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .conditions-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .conditions-list li {
    padding: 0.375rem 0.625rem;
    background: #f9fafb;
    border-radius: 4px;
    font-size: 0.8125rem;
    color: #374151;
    font-family: monospace;
  }

  .rule-meta {
    font-size: 0.75rem;
    color: #9ca3af;
    padding-top: 0.5rem;
    border-top: 1px solid #f3f4f6;
  }

  /* Loading & Error States */
  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    gap: 0.75rem;
    color: #6b7280;
  }

  .loading-state .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
  }

  .error-state svg {
    width: 24px;
    height: 24px;
    color: #ef4444;
  }

  .retry-btn {
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .retry-btn:hover {
    background: #2563eb;
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    text-align: center;
    color: #9ca3af;
  }

  .empty-state svg {
    width: 48px;
    height: 48px;
    margin-bottom: 1rem;
    color: #d1d5db;
  }

  .empty-state p {
    margin: 0.375rem 0;
    font-size: 0.875rem;
  }

  .empty-hint {
    color: #d1d5db;
    font-size: 0.8125rem;
  }
</style>
