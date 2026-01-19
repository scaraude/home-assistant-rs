<script lang="ts">
  import type {
    AutomationRule,
    DeviceInfo,
    SwitchDevice,
    CreateAutomationRuleRequest,
    UpdateAutomationRuleRequest,
  } from "../api";
  import { createAutomationRule, updateAutomationRule } from "../api";
  import { automationStore } from "../stores/automations";
  import { fade, scale } from "svelte/transition";

  let {
    rule = null,
    targetDeviceId,
    availableSensors = [],
    availableSwitches = [],
    onclose,
  }: {
    rule?: AutomationRule | null;
    targetDeviceId: string;
    availableSensors?: DeviceInfo[];
    availableSwitches?: SwitchDevice[];
    onclose: () => void;
  } = $props();
  const isEditing = rule !== null;

  // Form state
  let name = $state(rule?.name || "");
  let description = $state(rule?.description || "");
  let enabled = $state(rule?.enabled ?? true);
  let conditionOperator = $state<"and" | "or">(
    rule?.condition_operator || "and"
  );
  let timeWindowEnabled = $state(rule?.time_window?.enabled ?? false);
  let timeWindowStart = $state(rule?.time_window?.start_time || "");
  let timeWindowEnd = $state(rule?.time_window?.end_time || "");
  let limitDays = $state(!!rule?.time_window?.active_days?.length);
  let activeDays = $state<number[]>(rule?.time_window?.active_days ?? []);

  // Conditions
  interface ConditionForm {
    id: number;
    device_id: string;
    field: string;
    operator: string;
    value: number;
  }

  let conditionIdCounter = $state(rule?.conditions.length ?? 0);
  let conditions = $state<ConditionForm[]>(
    rule?.conditions.map((c, i) => ({
      id: i,
      device_id: c.device_id,
      field: c.field,
      operator: c.operator,
      value: c.value,
    })) || []
  );

  // Actions
  interface ActionForm {
    id: number;
    device_id: string;
    action: string;
  }

  let actionIdCounter = $state(
    rule?.actions.length ?? (targetDeviceId ? 1 : 0)
  );
  let actions = $state<ActionForm[]>(
    rule?.actions.map((a, i) => ({
      id: i,
      device_id: a.device_id,
      action: a.action,
    })) || [{ id: 0, device_id: targetDeviceId, action: "on" }]
  );

  let saving = $state(false);
  let error = $state<string | null>(null);

  // Opposite rule (creates a second rule with inverted condition and action)
  let createOppositeRule = $state(false);
  let oppositeValue = $state<number>(20);

  // Define field sets per sensor type
  const FIELD_OPTIONS_MAP: Record<string, Array<{ value: string; label: string }>> = {
    temp_humidity: [
      { value: "temperature", label: "Temperature (°C)" },
      { value: "humidity", label: "Humidity (%)" },
      { value: "battery", label: "Battery Level (%)" },
      { value: "link_quality", label: "Link Quality" },
    ],
    presence: [
      { value: "presence", label: "Occupancy" },
      { value: "illumination", label: "Light Level" },
      { value: "battery", label: "Battery Level (%)" },
      { value: "link_quality", label: "Link Quality" },
    ],
  };

  // Helper function to get available fields for a specific device_id
  function getFieldsForDevice(deviceId: string) {
    if (!deviceId) return [];
    const sensor = availableSensors.find((s) => s.device_id === deviceId);
    if (!sensor) return [];
    const options = new Map<string, { value: string; label: string }>();
    for (const cap of sensor.capabilities) {
      if (cap.type !== "sensor") continue;
      const fields = FIELD_OPTIONS_MAP[cap.sensor_type] || [];
      for (const field of fields) {
        options.set(field.value, field);
      }
    }
    return Array.from(options.values());
  }

  const operatorOptions = [
    { value: "equal", label: "=" },
    { value: "not_equal", label: "≠" },
    { value: "greater_than", label: ">" },
    { value: "greater_than_or_equal", label: "≥" },
    { value: "less_than", label: "<" },
    { value: "less_than_or_equal", label: "≤" },
  ];

  const actionOptions = [
    { value: "on", label: "Turn On" },
    { value: "off", label: "Turn Off" },
    { value: "toggle", label: "Toggle" },
  ];

  const dayOptions = [
    { value: 0, label: "Sun" },
    { value: 1, label: "Mon" },
    { value: 2, label: "Tue" },
    { value: 3, label: "Wed" },
    { value: 4, label: "Thu" },
    { value: 5, label: "Fri" },
    { value: 6, label: "Sat" },
  ];

  function getOppositeOperator(op: string): string {
    const opposites: Record<string, string> = {
      less_than: "greater_than",
      less_than_or_equal: "greater_than_or_equal",
      greater_than: "less_than",
      greater_than_or_equal: "less_than_or_equal",
      equal: "not_equal",
      not_equal: "equal",
    };
    return opposites[op] || op;
  }

  function getOppositeAction(action: string): string {
    const opposites: Record<string, string> = {
      on: "off",
      off: "on",
      toggle: "toggle",
    };
    return opposites[action] || action;
  }

  function addCondition() {
    const firstSensor = availableSensors[0];
    const defaultFields = firstSensor ? getFieldsForDevice(firstSensor.device_id) : [];
    const defaultField = defaultFields[0]?.value || "temperature";

    conditions = [
      ...conditions,
      {
        id: conditionIdCounter++,
        device_id: firstSensor?.device_id || "",
        field: defaultField,
        operator: "greater_than",
        value: 20,
      },
    ];
  }

  function removeCondition(id: number) {
    conditions = conditions.filter((c) => c.id !== id);
  }

  function addAction() {
    actions = [
      ...actions,
      {
        id: actionIdCounter++,
        device_id: targetDeviceId,
        action: "on",
      },
    ];
  }

  function removeAction(id: number) {
    actions = actions.filter((a) => a.id !== id);
  }

  function isValidTime(value: string) {
    const match = /^(\d{2}):(\d{2})$/.exec(value);
    if (!match) {
      return false;
    }
    const hours = Number(match[1]);
    const minutes = Number(match[2]);
    return hours >= 0 && hours <= 23 && minutes >= 0 && minutes <= 59;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    error = null;

    // Validation
    if (!name.trim()) {
      error = "Rule name is required";
      return;
    }

    if (conditions.length === 0) {
      error = "At least one condition is required";
      return;
    }

    if (actions.length === 0) {
      error = "At least one action is required";
      return;
    }

    // Validate all conditions have device_id
    if (conditions.some((c) => !c.device_id)) {
      error = "All conditions must have a sensor selected";
      return;
    }

    // Validate all actions have device_id
    if (actions.some((a) => !a.device_id)) {
      error = "All actions must have a switch selected";
      return;
    }

    if (timeWindowEnabled) {
      if (!isValidTime(timeWindowStart) || !isValidTime(timeWindowEnd)) {
        error = "Time window requires valid start and end times (HH:MM)";
        return;
      }

      if (limitDays && activeDays.length === 0) {
        error = "Select at least one active day or disable day filtering";
        return;
      }
    }

    saving = true;

    try {
      // Strip out the temporary id fields before sending to API
      const conditionsForApi = conditions.map(({ id, ...rest }) => rest);
      const actionsForApi = actions.map(({ id, ...rest }) => rest);
      const activeDaysPayload = limitDays
        ? activeDays.map((day) => Number(day))
        : undefined;

      if (isEditing && rule) {
        // Update existing rule
        const updates: UpdateAutomationRuleRequest = {
          name: name.trim(),
          description: description.trim() || undefined,
          enabled,
          condition_operator: conditionOperator,
          conditions: conditionsForApi,
          actions: actionsForApi,
          time_window: {
            enabled: timeWindowEnabled,
            start_time: timeWindowStart.trim() || undefined,
            end_time: timeWindowEnd.trim() || undefined,
            active_days: activeDaysPayload,
          },
        };

        const updated = await updateAutomationRule(rule.id, updates);
        automationStore.updateRule(rule.id, updated);
      } else {
        // Create main rule
        const newRule: CreateAutomationRuleRequest = {
          name: name.trim(),
          description: description.trim() || undefined,
          enabled,
          condition_operator: conditionOperator,
          conditions: conditionsForApi,
          actions: actionsForApi,
          time_window: {
            enabled: timeWindowEnabled,
            start_time: timeWindowStart.trim() || undefined,
            end_time: timeWindowEnd.trim() || undefined,
            active_days: activeDaysPayload,
          },
        };

        const created = await createAutomationRule(newRule);
        automationStore.addRule(created);

        // Create opposite rule if enabled
        if (createOppositeRule && conditions.length === 1) {
          const oppositeConditions = conditionsForApi.map((c) => ({
            ...c,
            operator: getOppositeOperator(c.operator),
            value: oppositeValue,
          }));
          const oppositeActions = actionsForApi.map((a) => ({
            ...a,
            action: getOppositeAction(a.action),
          }));

          const oppositeRule: CreateAutomationRuleRequest = {
            name: `${name.trim()} (opposite)`,
            description: description.trim()
              ? `${description.trim()} - Opposite rule`
              : "Opposite rule",
            enabled,
            condition_operator: conditionOperator,
            conditions: oppositeConditions,
            actions: oppositeActions,
            time_window: {
              enabled: timeWindowEnabled,
              start_time: timeWindowStart.trim() || undefined,
              end_time: timeWindowEnd.trim() || undefined,
              active_days: activeDaysPayload,
            },
          };

          const createdOpposite = await createAutomationRule(oppositeRule);
          automationStore.addRule(createdOpposite);
        }
      }

      onclose();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to save rule";
      console.error("Failed to save rule:", err);
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    onclose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }
</script>

<div
  class="modal-backdrop"
  onclick={handleBackdropClick}
  transition:fade={{ duration: 200 }}
  role="presentation"
>
  <div
    class="modal-container"
    transition:scale={{ duration: 200, start: 0.95 }}
  >
    <div class="modal-header">
      <h2>{isEditing ? "Edit Rule" : "Create New Rule"}</h2>
      <button
        class="close-btn"
        onclick={handleCancel}
        type="button"
        aria-label="Close"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path
            d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
          />
        </svg>
      </button>
    </div>

    <form onsubmit={handleSubmit}>
      <div class="modal-body">
        <!-- Basic Info -->
        <div class="form-section">
          <h3 class="section-title">Basic Information</h3>

          <div class="form-group">
            <label for="rule-name">
              Rule Name <span class="required">*</span>
            </label>
            <input
              id="rule-name"
              type="text"
              bind:value={name}
              placeholder="e.g., Turn on heater when cold"
              required
            />
          </div>

          <div class="form-group">
            <label for="rule-description">Description</label>
            <textarea
              id="rule-description"
              bind:value={description}
              placeholder="Optional description of what this rule does..."
              rows="2"
            ></textarea>
          </div>

          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={enabled} />
              <span class="checkbox-text">Enable this rule immediately</span>
            </label>
          </div>
        </div>

        <!-- Time Window -->
        <div class="form-section">
          <h3 class="section-title">Time Window</h3>
          <p class="section-description">
            Limit when this rule can trigger. Times use the browser's local
            timezone.
          </p>

          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={timeWindowEnabled} />
              <span class="checkbox-text">Enable time window</span>
            </label>
          </div>

          {#if timeWindowEnabled}
            <div class="time-window-grid">
              <div class="input-group">
                <label for="time-window-start">Start time</label>
                <input
                  id="time-window-start"
                  type="time"
                  bind:value={timeWindowStart}
                  required
                />
              </div>

              <div class="input-group">
                <label for="time-window-end">End time</label>
                <input
                  id="time-window-end"
                  type="time"
                  bind:value={timeWindowEnd}
                  required
                />
              </div>
            </div>

            <div class="form-group checkbox-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={limitDays} />
                <span class="checkbox-text">Limit to specific days</span>
              </label>
            </div>

            {#if limitDays}
              <div class="day-picker">
                {#each dayOptions as day (day.value)}
                  <label
                    class="day-pill"
                    class:active={activeDays.includes(day.value)}
                  >
                    <input
                      type="checkbox"
                      bind:group={activeDays}
                      value={day.value}
                    />
                    <span>{day.label}</span>
                  </label>
                {/each}
              </div>
            {/if}
          {/if}
        </div>

        <!-- Conditions -->
        <div class="form-section">
          <div class="section-header">
            <h3 class="section-title">
              Conditions <span class="required">*</span>
            </h3>
            <div class="operator-toggle">
              <button
                type="button"
                class="operator-btn"
                class:active={conditionOperator === "and"}
                onclick={() => (conditionOperator = "and")}
              >
                AND
              </button>
              <button
                type="button"
                class="operator-btn"
                class:active={conditionOperator === "or"}
                onclick={() => (conditionOperator = "or")}
              >
                OR
              </button>
            </div>
          </div>

          <p class="section-description">
            {conditionOperator === "and"
              ? "All conditions must be true for the rule to trigger"
              : "Any condition being true will trigger the rule"}
          </p>

          <div class="conditions-list">
            {#each conditions as condition (condition.id)}
              <div class="condition-row">
                <div class="condition-inputs">
                  <div class="input-group">
                    <label for="condition-sensor-{condition.id}">Sensor</label>
                    <select
                      id="condition-sensor-{condition.id}"
                      bind:value={condition.device_id}
                      required
                    >
                      <option value="">Select sensor...</option>
                      {#each availableSensors as sensor (sensor.device_id)}
                        <option value={sensor.device_id}>
                          {sensor.name || sensor.device_id}
                        </option>
                      {/each}
                    </select>
                  </div>

                  <div class="input-group">
                    <label for="condition-field-{condition.id}">Field</label>
                    <select
                      id="condition-field-{condition.id}"
                      bind:value={condition.field}
                      required
                    >
                      {#each getFieldsForDevice(condition.device_id) as field (field.value)}
                        <option value={field.value}>{field.label}</option>
                      {/each}
                    </select>
                  </div>

                  <div class="input-group">
                    <label for="condition-operator-{condition.id}"
                      >Operator</label
                    >
                    <select
                      id="condition-operator-{condition.id}"
                      bind:value={condition.operator}
                      required
                    >
                      {#each operatorOptions as op (op.value)}
                        <option value={op.value}>{op.label}</option>
                      {/each}
                    </select>
                  </div>

                  <div class="input-group">
                    <label for="condition-value-{condition.id}">Value</label>

                    {#if condition.field === 'illumination'}
                      <!-- Special dropdown for illumination: "Dim" (0) or "Bright" (1) -->
                      <select
                        id="condition-value-{condition.id}"
                        bind:value={condition.value}
                        required
                      >
                        <option value={0}>Dim</option>
                        <option value={1}>Bright</option>
                      </select>
                    {:else if condition.field === 'presence'}
                      <!-- Special dropdown for presence: "Not Occupied" (0) or "Occupied" (1) -->
                      <select
                        id="condition-value-{condition.id}"
                        bind:value={condition.value}
                        required
                      >
                        <option value={0}>Not Occupied</option>
                        <option value={1}>Occupied</option>
                      </select>
                    {:else}
                      <!-- Regular numeric input for temperature, humidity, battery, link_quality -->
                      <input
                        id="condition-value-{condition.id}"
                        type="number"
                        step={condition.field === 'temperature' ||
                        condition.field === 'humidity'
                          ? '0.1'
                          : '1'}
                        bind:value={condition.value}
                        required
                      />
                    {/if}
                  </div>
                </div>

                <button
                  type="button"
                  class="remove-btn"
                  onclick={() => removeCondition(condition.id)}
                  title="Remove condition"
                  aria-label="Remove condition"
                  disabled={conditions.length === 1}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
                    />
                  </svg>
                </button>
              </div>
            {/each}
          </div>

          <button type="button" class="add-btn" onclick={addCondition}>
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
            </svg>
            Add Condition
          </button>
        </div>

        <!-- Actions -->
        <div class="form-section">
          <h3 class="section-title">
            Actions <span class="required">*</span>
          </h3>
          <p class="section-description">
            What should happen when the conditions are met?
          </p>

          <div class="actions-list">
            {#each actions as action (action.id)}
              <div class="action-row">
                <div class="action-inputs">
                  <div class="input-group">
                    <label for="action-switch-{action.id}">Switch</label>
                    <select
                      id="action-switch-{action.id}"
                      bind:value={action.device_id}
                      required
                    >
                      <option value="">Select switch...</option>
                      {#each availableSwitches as sw (sw.id)}
                        <option value={sw.id}>{sw.name || sw.id}</option>
                      {/each}
                    </select>
                  </div>

                  <div class="input-group">
                    <label for="action-type-{action.id}">Action</label>
                    <select
                      id="action-type-{action.id}"
                      bind:value={action.action}
                      required
                    >
                      {#each actionOptions as act (act.value)}
                        <option value={act.value}>{act.label}</option>
                      {/each}
                    </select>
                  </div>
                </div>

                <button
                  type="button"
                  class="remove-btn"
                  onclick={() => removeAction(action.id)}
                  title="Remove action"
                  aria-label="Remove action"
                  disabled={actions.length === 1}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
                    />
                  </svg>
                </button>
              </div>
            {/each}
          </div>

          <button type="button" class="add-btn" onclick={addAction}>
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
            </svg>
            Add Action
          </button>
        </div>

        <!-- Opposite Rule Option (only for new rules with single condition) -->
        {#if !isEditing && conditions.length === 1}
          <div class="form-section opposite-rule-section">
            <h3 class="section-title">Opposite Trigger</h3>
            <p class="section-description">
              Create a second rule that does the opposite action at a different
              threshold. For example: Turn ON heater at 16°C, turn OFF at 20°C.
            </p>

            <div class="form-group checkbox-group">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={createOppositeRule} />
                <span class="checkbox-text">Create opposite rule</span>
              </label>
            </div>

            {#if createOppositeRule}
              <div class="opposite-rule-preview">
                <div class="preview-header">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path
                      d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74C4.46 8.97 4 10.43 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z"
                    />
                  </svg>
                  <span>Opposite rule will:</span>
                </div>
                <div class="preview-content">
                  <div class="preview-item">
                    <span class="preview-label">Condition:</span>
                    <span class="preview-value">
                      {conditions[0]?.field || "field"}
                      {operatorOptions.find(
                        (o) =>
                          o.value ===
                          getOppositeOperator(conditions[0]?.operator)
                      )?.label || "?"}
                    </span>
                    <input
                      type="number"
                      step="0.1"
                      bind:value={oppositeValue}
                      class="opposite-value-input"
                    />
                  </div>
                  <div class="preview-item">
                    <span class="preview-label">Action:</span>
                    <span
                      class="preview-value action-badge"
                      class:action-on={getOppositeAction(actions[0]?.action) ===
                        "on"}
                      class:action-off={getOppositeAction(
                        actions[0]?.action
                      ) === "off"}
                    >
                      {actionOptions.find(
                        (a) => a.value === getOppositeAction(actions[0]?.action)
                      )?.label || "?"}
                    </span>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {/if}

        {#if error}
          <div class="error-message" transition:fade={{ duration: 150 }}>
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
      </div>

      <div class="modal-footer">
        <button type="button" class="cancel-btn" onclick={handleCancel}>
          Cancel
        </button>
        <button type="submit" class="save-btn" disabled={saving}>
          {#if saving}
            <div class="spinner"></div>
            Saving...
          {:else}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path
                d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"
              />
            </svg>
            {isEditing ? "Update Rule" : "Create Rule"}
          {/if}
        </button>
      </div>
    </form>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
  }

  .modal-container {
    background: white;
    border-radius: 12px;
    box-shadow:
      0 20px 25px -5px rgba(0, 0, 0, 0.1),
      0 10px 10px -5px rgba(0, 0, 0, 0.04);
    max-width: 800px;
    width: 100%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-container form {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.5rem;
    border-bottom: 2px solid #e5e7eb;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: #111827;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .close-btn:hover {
    background: #f3f4f6;
    color: #111827;
  }

  .close-btn svg {
    width: 20px;
    height: 20px;
  }

  .modal-body {
    overflow-y: auto;
    overflow-x: hidden;
    padding: 1.5rem;
    flex: 1;
    min-height: 0;
  }

  /* Custom scrollbar for modal body */
  .modal-body::-webkit-scrollbar {
    width: 8px;
  }

  .modal-body::-webkit-scrollbar-track {
    background: #f3f4f6;
    border-radius: 4px;
  }

  .modal-body::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 4px;
  }

  .modal-body::-webkit-scrollbar-thumb:hover {
    background: #94a3b8;
  }

  .form-section {
    margin-bottom: 2rem;
  }

  .form-section:last-child {
    margin-bottom: 0;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .section-title {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 700;
    color: #111827;
  }

  .section-description {
    margin: 0 0 1rem;
    font-size: 0.875rem;
    color: #6b7280;
    line-height: 1.5;
  }

  .required {
    color: #ef4444;
  }

  .operator-toggle {
    display: flex;
    background: #f3f4f6;
    border-radius: 6px;
    padding: 2px;
  }

  .operator-btn {
    padding: 0.375rem 1rem;
    background: transparent;
    border: none;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    color: #6b7280;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .operator-btn.active {
    background: white;
    color: #3b82f6;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .form-group {
    margin-bottom: 1.25rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: #374151;
  }

  .form-group input[type="text"],
  .form-group textarea {
    width: 100%;
    padding: 0.75rem;
    border: 2px solid #e5e7eb;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #111827;
    transition: all 0.2s ease;
    font-family: inherit;
  }

  .form-group input[type="text"]:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .form-group textarea {
    resize: vertical;
  }

  .checkbox-group {
    margin-bottom: 0;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: #3b82f6;
  }

  .checkbox-text {
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
  }

  .conditions-list,
  .actions-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .condition-row,
  .action-row {
    display: flex;
    gap: 0.75rem;
    padding: 1rem;
    background: #f9fafb;
    border: 2px solid #e5e7eb;
    border-radius: 8px;
  }

  .condition-inputs,
  .action-inputs {
    flex: 1;
    display: grid;
    gap: 0.75rem;
  }

  .condition-inputs {
    grid-template-columns: 2fr 1fr 1fr 1fr;
  }

  .action-inputs {
    grid-template-columns: 2fr 1fr;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .input-group label {
    font-size: 0.75rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .input-group select,
  .input-group input[type="number"],
  .input-group input[type="time"] {
    padding: 0.625rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #111827;
    background: white;
    transition: all 0.2s ease;
  }

  .input-group select:focus,
  .input-group input[type="number"]:focus,
  .input-group input[type="time"]:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .time-window-grid {
    display: grid;
    gap: 0.75rem;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    margin-bottom: 1.25rem;
  }

  .day-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .day-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.4rem 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 999px;
    background: white;
    color: #374151;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .day-pill input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .day-pill:hover {
    border-color: #94a3b8;
    color: #111827;
  }

  .day-pill.active {
    background: #eff6ff;
    border-color: #3b82f6;
    color: #1d4ed8;
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    align-self: flex-end;
    padding: 0;
    background: white;
    border: 1px solid #fecaca;
    border-radius: 6px;
    color: #dc2626;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .remove-btn:hover:not(:disabled) {
    background: #fef2f2;
    border-color: #f87171;
  }

  .remove-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .remove-btn svg {
    width: 18px;
    height: 18px;
  }

  .add-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: white;
    border: 2px dashed #cbd5e1;
    border-radius: 8px;
    color: #475569;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    width: 100%;
    justify-content: center;
  }

  .add-btn:hover {
    background: #f8fafc;
    border-color: #94a3b8;
    color: #334155;
  }

  .add-btn svg {
    width: 18px;
    height: 18px;
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 1rem;
    background: #fef2f2;
    border: 2px solid #fecaca;
    border-radius: 8px;
    color: #dc2626;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .error-message svg {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1.25rem 1.5rem;
    border-top: 2px solid #e5e7eb;
    flex-shrink: 0;
    background: white;
  }

  .cancel-btn,
  .save-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .cancel-btn {
    background: white;
    border: 2px solid #e5e7eb;
    color: #6b7280;
  }

  .cancel-btn:hover {
    background: #f9fafb;
    border-color: #cbd5e1;
  }

  .save-btn {
    background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
    color: white;
    box-shadow: 0 4px 6px rgba(59, 130, 246, 0.2);
  }

  .save-btn:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 6px 12px rgba(59, 130, 246, 0.3);
  }

  .save-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .save-btn svg {
    width: 18px;
    height: 18px;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Opposite Rule Section */
  .opposite-rule-section {
    background: linear-gradient(135deg, #fef3c7 0%, #fde68a 100%);
    border: 2px solid #f59e0b;
    border-radius: 8px;
    padding: 1rem;
  }

  .opposite-rule-section .section-title {
    color: #92400e;
  }

  .opposite-rule-section .section-description {
    color: #a16207;
  }

  .opposite-rule-preview {
    background: white;
    border: 1px solid #fcd34d;
    border-radius: 8px;
    padding: 1rem;
    margin-top: 1rem;
  }

  .preview-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 600;
    color: #92400e;
    margin-bottom: 0.75rem;
  }

  .preview-header svg {
    width: 20px;
    height: 20px;
    color: #f59e0b;
  }

  .preview-content {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .preview-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .preview-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: #6b7280;
    min-width: 80px;
  }

  .preview-value {
    font-size: 0.875rem;
    color: #374151;
    font-weight: 500;
  }

  .opposite-value-input {
    width: 80px;
    padding: 0.375rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    font-size: 0.875rem;
    color: #111827;
    background: white;
  }

  .opposite-value-input:focus {
    outline: none;
    border-color: #f59e0b;
    box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.1);
  }

  .action-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .action-badge.action-on {
    background: #dcfce7;
    color: #166534;
  }

  .action-badge.action-off {
    background: #fee2e2;
    color: #991b1b;
  }

  @media (max-width: 768px) {
    .condition-inputs {
      grid-template-columns: 1fr;
    }

    .action-inputs {
      grid-template-columns: 1fr;
    }

    .time-window-grid {
      grid-template-columns: 1fr;
    }

    .modal-container {
      max-height: 95vh;
    }
  }
</style>
