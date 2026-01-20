<script lang="ts">
  import type { SwitchDevice, AutomationRule, DeviceInfo } from "../api";
  import {
    deleteAutomationRule,
    updateAutomationRule,
    fetchSensors,
    fetchSwitches,
  } from "../api";
  import { automationStore } from "../stores/automations";
  import { formatDistanceToNow } from "date-fns";
  import { slide, fade } from "svelte/transition";
  import RuleEditor from "./RuleEditor.svelte";

  let {
    deviceId,
    deviceRules = [],
    loading = false,
    error = null,
  }: {
    deviceId: string;
    deviceRules?: AutomationRule[];
    loading?: boolean;
    error?: string | null;
  } = $props();

  let showEditor = $state(false);
  let editingRule = $state<AutomationRule | null>(null);
  let deletingRuleId = $state<string | null>(null);
  let confirmDelete = $state<string | null>(null);

  // Available devices for rule creation
  let availableSensors = $state<DeviceInfo[]>([]);
  let availableSwitches = $state<SwitchDevice[]>([]);

  async function handleCreateRule() {
    // Load available devices if not already loaded
    if (availableSensors.length === 0) {
      try {
        [availableSensors, availableSwitches] = await Promise.all([
          fetchSensors(),
          fetchSwitches(),
        ]);
      } catch (err) {
        console.error("Failed to load devices:", err);
      }
    }

    editingRule = null;
    showEditor = true;
  }

  async function handleEditRule(rule: AutomationRule) {
    // Load available devices if not already loaded
    if (availableSensors.length === 0) {
      try {
        [availableSensors, availableSwitches] = await Promise.all([
          fetchSensors(),
          fetchSwitches(),
        ]);
      } catch (err) {
        console.error("Failed to load devices:", err);
      }
    }

    editingRule = rule;
    showEditor = true;
  }

  function handleCloseEditor() {
    showEditor = false;
    editingRule = null;
  }

  async function handleToggleEnabled(rule: AutomationRule) {
    try {
      const updated = await updateAutomationRule(rule.id, {
        enabled: !rule.enabled,
      });
      automationStore.updateRule(rule.id, updated);
    } catch (err) {
      console.error("Failed to toggle rule:", err);
      alert(err instanceof Error ? err.message : "Failed to toggle rule");
    }
  }

  async function handleDeleteRule(ruleId: string) {
    if (confirmDelete !== ruleId) {
      confirmDelete = ruleId;
      setTimeout(() => {
        confirmDelete = null;
      }, 3000);
      return;
    }

    deletingRuleId = ruleId;
    try {
      await deleteAutomationRule(ruleId);
      automationStore.removeRule(ruleId);
      confirmDelete = null;
    } catch (err) {
      console.error("Failed to delete rule:", err);
      alert(err instanceof Error ? err.message : "Failed to delete rule");
    } finally {
      deletingRuleId = null;
    }
  }

  const dayLabels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

  function formatActiveDays(days: number[] | null) {
    if (!days || days.length === 0) {
      return "Every day";
    }
    const unique = Array.from(new Set(days)).sort((a, b) => a - b);
    return unique.map((day) => dayLabels[day] ?? String(day)).join(", ");
  }

  function formatTimeWindow(rule: AutomationRule) {
    if (!rule.time_window?.enabled) {
      return "Always active";
    }

    const start = rule.time_window.start_time || "??:??";
    const end = rule.time_window.end_time || "??:??";
    const daysLabel = formatActiveDays(rule.time_window.active_days);
    return `${start}–${end} · ${daysLabel}`;
  }

  function formatTimeAgo(timestamp: Date) {
    const timeAgo = formatDistanceToNow(timestamp, {
      addSuffix: true,
    });
    return timeAgo === "less than a minute ago" ? "now" : timeAgo;
  }
</script>

{#if showEditor}
  <RuleEditor
    rule={editingRule}
    targetDeviceId={deviceId}
    {availableSensors}
    {availableSwitches}
    onclose={handleCloseEditor}
  />
{/if}

<div class="automation-panel">
  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <span>Loading rules...</span>
    </div>
  {:else if error}
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
      <span>{error}</span>
    </div>
  {:else}
    <div class="panel-header">
      <h3>Automation Rules</h3>
      <button class="create-btn" onclick={handleCreateRule} type="button">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
        </svg>
        Create Rule
      </button>
    </div>

    {#if deviceRules.length === 0}
      <div class="empty-state" transition:fade={{ duration: 150 }}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path
            d="M22.7 19l-9.1-9.1c.9-2.3.4-5-1.5-6.9-2-2-5-2.4-7.4-1.3L9 6 6 9 1.6 4.7C.4 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1.1.1-1.4z"
          />
        </svg>
        <p class="empty-title">No automation rules yet</p>
        <p class="empty-hint">
          Create a rule to automate this switch based on sensor readings
        </p>
        <button
          class="empty-action-btn"
          onclick={handleCreateRule}
          type="button"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
          </svg>
          Create Your First Rule
        </button>
      </div>
    {:else}
      <div class="rules-list">
        {#each deviceRules as rule (rule.id)}
          <div
            class="rule-card"
            class:disabled={!rule.enabled}
            transition:slide={{ duration: 200 }}
          >
            <div class="rule-header">
              <div class="rule-info">
                <h4 class="rule-name">{rule.name}</h4>
                {#if rule.description}
                  <p class="rule-description">{rule.description}</p>
                {/if}
              </div>
              <div class="rule-controls">
                <button
                  class="toggle-enabled-btn"
                  class:enabled={rule.enabled}
                  onclick={() => handleToggleEnabled(rule)}
                  title={rule.enabled ? "Disable rule" : "Enable rule"}
                  type="button"
                >
                  <div class="toggle-switch" class:on={rule.enabled}>
                    <div class="toggle-slider"></div>
                  </div>
                  <span>{rule.enabled ? "Enabled" : "Disabled"}</span>
                </button>
              </div>
            </div>

            <div class="rule-body">
              <div class="rule-section">
                <div class="section-header">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M10 3H4c-.55 0-1 .45-1 1v6c0 .55.45 1 1 1h6c.55 0 1-.45 1-1V4c0-.55-.45-1-1-1zM10 13H4c-.55 0-1 .45-1 1v6c0 .55.45 1 1 1h6c.55 0 1-.45 1-1v-6c0-.55-.45-1-1-1zM20 3h-6c-.55 0-1 .45-1 1v6c0 .55.45 1 1 1h6c.55 0 1-.45 1-1V4c0-.55-.45-1-1-1zM20 13h-6c-.55 0-1 .45-1 1v6c0 .55.45 1 1 1h6c.55 0 1-.45 1-1v-6c0-.55-.45-1-1-1z"
                    />
                  </svg>
                  <span class="section-label">
                    Conditions ({rule.condition_operator.toUpperCase()})
                  </span>
                </div>
                <ul class="conditions-list">
                  {#each rule.conditions as condition}
                    <li class="condition-item">
                      <span class="condition-field">{condition.field}</span>
                      <span class="condition-operator">
                        {condition.operator.replace(/_/g, " ")}
                      </span>
                      <span class="condition-value">{condition.value}</span>
                    </li>
                  {/each}
                </ul>
              </div>

              <div class="rule-section">
                <div class="section-header">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M1 21h4V9H1v12zm22-11c0-1.1-.9-2-2-2h-6.31l.95-4.57.03-.32c0-.41-.17-.79-.44-1.06L14.17 1 7.59 7.59C7.22 7.95 7 8.45 7 9v10c0 1.1.9 2 2 2h9c.83 0 1.54-.5 1.84-1.22l3.02-7.05c.09-.23.14-.47.14-.73v-2z"
                    />
                  </svg>
                  <span class="section-label">Actions</span>
                </div>
                <ul class="actions-list">
                  {#each rule.actions as action}
                    <li class="action-item">
                      <span
                        class="action-type"
                        class:on={action.action === "on"}
                      >
                        {action.action.toUpperCase()}
                      </span>
                    </li>
                  {/each}
                </ul>
              </div>

              <div class="rule-section">
                <div class="section-header">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M12 7a1 1 0 0 1 1 1v4.38l2.44 1.41a1 1 0 1 1-1 1.74l-2.94-1.7A1 1 0 0 1 11 13V8a1 1 0 0 1 1-1zm0-5a10 10 0 1 1 0 20 10 10 0 0 1 0-20zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16z"
                    />
                  </svg>
                  <span class="section-label">Time Window</span>
                </div>
                <div class="time-window-summary">{formatTimeWindow(rule)}</div>
              </div>

              {#if rule.last_triggered_at}
                <div class="rule-meta">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z"
                    />
                  </svg>
                  <span>
                    Last triggered {formatTimeAgo(rule.last_triggered_at)}
                  </span>
                  <span class="trigger-count">
                    {rule.trigger_count}
                    {rule.trigger_count === 1 ? "execution" : "executions"}
                  </span>
                </div>
              {/if}
            </div>

            <div class="rule-actions">
              <button
                class="edit-btn"
                onclick={() => handleEditRule(rule)}
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
                Edit
              </button>
              <button
                class="delete-btn"
                class:confirming={confirmDelete === rule.id}
                onclick={() => handleDeleteRule(rule.id)}
                disabled={deletingRuleId === rule.id}
                type="button"
              >
                {#if deletingRuleId === rule.id}
                  <div class="spinner small"></div>
                  Deleting...
                {:else if confirmDelete === rule.id}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
                    />
                  </svg>
                  Click again to confirm
                {:else}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"
                    />
                  </svg>
                  Delete
                {/if}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .automation-panel {
    background: #fafbfc;
    border-radius: 8px;
    padding: 1rem;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 2px solid #e5e7eb;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: #111827;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .create-btn {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 1rem;
    background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 4px rgba(59, 130, 246, 0.2);
  }

  .create-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(59, 130, 246, 0.3);
  }

  .create-btn:active {
    transform: translateY(0);
  }

  .create-btn svg {
    width: 18px;
    height: 18px;
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

  .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .spinner.small {
    width: 16px;
    height: 16px;
    border-width: 2px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-state svg {
    width: 24px;
    height: 24px;
    color: #ef4444;
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 2rem;
    text-align: center;
  }

  .empty-state svg {
    width: 64px;
    height: 64px;
    color: #d1d5db;
    margin-bottom: 1rem;
  }

  .empty-title {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    color: #6b7280;
  }

  .empty-hint {
    margin: 0 0 1.5rem;
    font-size: 0.875rem;
    color: #9ca3af;
    max-width: 320px;
  }

  .empty-action-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 6px rgba(59, 130, 246, 0.2);
  }

  .empty-action-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 12px rgba(59, 130, 246, 0.3);
  }

  .empty-action-btn svg {
    width: 20px;
    height: 20px;
  }

  /* Rules List */
  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* Rule Card */
  .rule-card {
    background: white;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
    transition: all 0.2s ease;
  }

  .rule-card:hover {
    border-color: #cbd5e1;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.08);
  }

  .rule-card.disabled {
    opacity: 0.6;
    background: #f9fafb;
  }

  .rule-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 1rem;
    gap: 1rem;
  }

  .rule-info {
    flex: 1;
    min-width: 0;
  }

  .rule-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: #111827;
  }

  .rule-description {
    margin: 0.375rem 0 0;
    font-size: 0.875rem;
    color: #6b7280;
    line-height: 1.5;
  }

  .rule-controls {
    display: flex;
    align-items: center;
  }

  .toggle-enabled-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 600;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .toggle-enabled-btn:hover {
    background: #f9fafb;
    border-color: #cbd5e1;
  }

  .toggle-enabled-btn.enabled {
    color: #16a34a;
  }

  .toggle-switch {
    position: relative;
    width: 36px;
    height: 20px;
    background: #d1d5db;
    border-radius: 10px;
    transition: background 0.3s ease;
  }

  .toggle-switch.on {
    background: #16a34a;
  }

  .toggle-slider {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    background: white;
    border-radius: 50%;
    transition: transform 0.3s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .toggle-switch.on .toggle-slider {
    transform: translateX(16px);
  }

  /* Rule Body */
  .rule-body {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .rule-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .section-header svg {
    width: 16px;
    height: 16px;
    color: #6b7280;
  }

  .section-label {
    font-size: 0.75rem;
    font-weight: 700;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .conditions-list,
  .actions-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .condition-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 0.75rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 0.8125rem;
  }

  .condition-field {
    font-weight: 600;
    color: #3b82f6;
    text-transform: capitalize;
  }

  .condition-operator {
    color: #6b7280;
    font-style: italic;
  }

  .condition-value {
    font-weight: 700;
    color: #111827;
    font-family: monospace;
  }

  .action-item {
    display: inline-flex;
    align-items: center;
    padding: 0.5rem 0.875rem;
    background: #f3f4f6;
    border-radius: 6px;
    width: fit-content;
  }

  .action-type {
    font-size: 0.8125rem;
    font-weight: 700;
    color: #6b7280;
  }

  .action-type.on {
    color: #16a34a;
  }

  .time-window-summary {
    display: inline-flex;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: #f8fafc;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 600;
    color: #475569;
    width: fit-content;
  }

  .rule-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding-top: 0.75rem;
    border-top: 1px solid #e5e7eb;
    font-size: 0.75rem;
    color: #9ca3af;
  }

  .rule-meta svg {
    width: 14px;
    height: 14px;
  }

  .trigger-count {
    margin-left: auto;
    padding: 0.25rem 0.5rem;
    background: #f3f4f6;
    border-radius: 4px;
    font-weight: 600;
    color: #6b7280;
  }

  /* Rule Actions */
  .rule-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid #e5e7eb;
  }

  .edit-btn,
  .delete-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    padding: 0.625rem 1rem;
    border: 1px solid;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .edit-btn {
    background: white;
    border-color: #cbd5e1;
    color: #475569;
  }

  .edit-btn:hover {
    background: #f8fafc;
    border-color: #94a3b8;
    transform: translateY(-1px);
  }

  .edit-btn svg {
    width: 16px;
    height: 16px;
  }

  .delete-btn {
    background: white;
    border-color: #fecaca;
    color: #dc2626;
  }

  .delete-btn:hover {
    background: #fef2f2;
    border-color: #f87171;
    transform: translateY(-1px);
  }

  .delete-btn.confirming {
    background: #fee2e2;
    border-color: #ef4444;
    animation: pulse 1s ease-in-out;
  }

  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.02);
    }
  }

  .delete-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .delete-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Responsive styles */
  @media (max-width: 768px) {
    .automation-panel {
      padding: 0.75rem;
    }

    .panel-header h3 {
      font-size: 0.9375rem;
    }

    .create-btn {
      padding: 0.4rem 0.75rem;
      font-size: 0.8125rem;
    }

    .rule-card {
      padding: 0.75rem;
    }

    .rule-header {
      flex-direction: column;
      gap: 0.75rem;
    }

    .rule-controls {
      width: 100%;
    }

    .toggle-enabled-btn {
      width: 100%;
      justify-content: center;
    }

    .empty-state {
      padding: 2rem 1rem;
    }

    .empty-state svg {
      width: 48px;
      height: 48px;
    }
  }

  @media (max-width: 480px) {
    .automation-panel {
      padding: 0.5rem;
    }

    .panel-header {
      flex-direction: column;
      align-items: stretch;
      gap: 0.75rem;
    }

    .create-btn {
      width: 100%;
      justify-content: center;
    }

    .rule-card {
      padding: 0.625rem;
    }

    .rule-name {
      font-size: 0.9375rem;
    }

    .rule-description {
      font-size: 0.8125rem;
    }

    .condition-item {
      flex-wrap: wrap;
      padding: 0.5rem;
      font-size: 0.75rem;
    }

    .rule-actions {
      flex-direction: column;
    }

    .edit-btn,
    .delete-btn {
      width: 100%;
    }

    .rule-meta {
      flex-wrap: wrap;
      font-size: 0.6875rem;
    }

    .trigger-count {
      margin-left: 0;
      margin-top: 0.25rem;
      width: 100%;
      text-align: center;
    }

    .empty-state {
      padding: 1.5rem 0.75rem;
    }

    .empty-hint {
      font-size: 0.8125rem;
    }

    .empty-action-btn {
      width: 100%;
      justify-content: center;
      padding: 0.625rem 1rem;
    }
  }
</style>
