<script lang="ts">
  import { updateDeviceName } from "../api";
  import { dataCache } from "../stores/dataCache";

  let {
    deviceId,
    name,
    class: className = "",
    isEdit,
    onSaved,
  }: {
    deviceId: string;
    name: string;
    class?: string;
    /** If provided, controls editing state externally. If undefined, component manages its own state. */
    isEdit?: boolean;
    onSaved?: () => void;
  } = $props();

  // Internal editing state (used when isEdit is undefined)
  let internalIsEditing = $state(false);
  let editedName = $state("");
  let error = $state<string | null>(null);
  let isSaving = $state(false);

  // Determine if we're in controlled mode
  let isControlled = $derived(isEdit !== undefined);
  // The actual editing state: use prop if controlled, internal state otherwise
  let isEditing = $derived(isControlled ? isEdit : internalIsEditing);

  // In controlled mode, initialize editedName when isEdit becomes true
  $effect(() => {
    if (isControlled && isEdit) {
      editedName = name;
      error = null;
    }
  });

  function startEditing() {
    editedName = name;
    if (!isControlled) {
      internalIsEditing = true;
    }
    error = null;
  }

  async function save() {
    const trimmed = editedName.trim();

    if (!trimmed) {
      error = "Name cannot be empty";
      return;
    }

    if (trimmed === name) {
      if (!isControlled) {
        internalIsEditing = false;
      }
      onSaved?.();
      return;
    }

    isSaving = true;
    error = null;

    try {
      await updateDeviceName(deviceId, trimmed);
      dataCache.updateDeviceName(deviceId, trimmed);
      if (!isControlled) {
        internalIsEditing = false;
      }
      onSaved?.();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to save";
      console.error("Failed to update device name:", err);
    } finally {
      isSaving = false;
    }
  }

  function cancel() {
    if (!isControlled) {
      internalIsEditing = false;
    }
    error = null;
    onSaved?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      void save();
    } else if (e.key === "Escape") {
      cancel();
    }
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

{#if isEditing}
  <input
    type="text"
    class="name-input {className}"
    bind:value={editedName}
    onkeydown={handleKeydown}
    onblur={() => save()}
    onclick={(e) => e.stopPropagation()}
    disabled={isSaving}
    use:focusOnMount
  />
  {#if error}
    <span class="error-text">{error}</span>
  {/if}
{:else if isControlled}
  <!-- Controlled mode: just display text (parent controls when to edit) -->
  <span class="name-text {className}" title={deviceId}>{name}</span>
{:else}
  <!-- Uncontrolled mode: clickable button to enter edit -->
  <button
    class="name-button {className}"
    title={deviceId}
    onclick={(e) => { e.stopPropagation(); startEditing(); }}
    type="button"
  >
    {name}
  </button>
{/if}

<style>
  .name-button,
  .name-text {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name-button {
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    transition: color 0.2s;
  }

  .name-button:hover {
    color: #3b82f6;
  }

  .name-input {
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

  .name-input:focus {
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .name-input:disabled {
    opacity: 0.6;
  }

  .error-text {
    font-size: 0.75rem;
    color: #dc2626;
    display: block;
    margin-top: 0.25rem;
  }
</style>
