<script lang="ts">
  import { RECOMMENDED_COLORS } from "./stores/graphConfig";

  let {
    currentColor,
    onSelect,
    onClose
  }: {
    currentColor: string;
    onSelect: (color: string) => void;
    onClose: () => void;
  } = $props();

  let customColor = $state(currentColor);

  function handleColorSelect(color: string) {
    onSelect(color);
    onClose();
  }

  function handleCustomColorSubmit() {
    if (customColor && /^#[0-9A-Fa-f]{6}$/.test(customColor)) {
      onSelect(customColor);
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-backdrop" onclick={handleBackdropClick} role="presentation">
  <div class="modal-content" role="dialog" aria-label="Color picker">
    <div class="modal-header">
      <h3>Choose a color</h3>
      <button class="close-btn" onclick={onClose} aria-label="Close">×</button>
    </div>

    <div class="modal-body">
      <div class="color-grid">
        {#each RECOMMENDED_COLORS as color}
          <button
            class="color-swatch"
            class:selected={color === currentColor}
            style="background-color: {color}"
            onclick={() => handleColorSelect(color)}
            aria-label="Select color {color}"
          >
            {#if color === currentColor}
              <span class="checkmark">✓</span>
            {/if}
          </button>
        {/each}
      </div>

      <div class="custom-color">
        <label for="custom-color-input">Custom color (hex):</label>
        <div class="custom-color-input-group">
          <input
            id="custom-color-input"
            type="text"
            bind:value={customColor}
            placeholder="#3b82f6"
            pattern="^#[0-9A-Fa-f]{6}$"
          />
          <button
            class="apply-btn"
            onclick={handleCustomColorSubmit}
            disabled={!customColor || !/^#[0-9A-Fa-f]{6}$/.test(customColor)}
          >
            Apply
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .modal-content {
    background: white;
    border-radius: 12px;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1),
      0 10px 10px -5px rgba(0, 0, 0, 0.04);
    max-width: 360px;
    width: 90%;
    animation: slideUp 0.2s ease-out;
  }

  @keyframes slideUp {
    from {
      transform: translateY(20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.75rem;
    color: #6b7280;
    cursor: pointer;
    padding: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: #f3f4f6;
    color: #111827;
  }

  .modal-body {
    padding: 1.5rem;
  }

  .color-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  .color-swatch {
    aspect-ratio: 1;
    border: 3px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .color-swatch:hover {
    transform: scale(1.1);
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.15);
  }

  .color-swatch.selected {
    border-color: #111827;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.2);
  }

  .checkmark {
    color: white;
    font-size: 1.5rem;
    font-weight: bold;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .custom-color {
    padding-top: 1rem;
    border-top: 1px solid #e5e7eb;
  }

  .custom-color label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
    margin-bottom: 0.5rem;
  }

  .custom-color-input-group {
    display: flex;
    gap: 0.5rem;
  }

  .custom-color-input-group input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    font-family: monospace;
    transition: all 0.15s;
  }

  .custom-color-input-group input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .custom-color-input-group input:invalid {
    border-color: #ef4444;
  }

  .apply-btn {
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .apply-btn:hover:not(:disabled) {
    background: #2563eb;
  }

  .apply-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
