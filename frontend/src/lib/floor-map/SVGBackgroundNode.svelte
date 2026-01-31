<script lang="ts">
  import type { Node, NodeProps } from "@xyflow/svelte";

  type SVGBackgroundData = Record<string, unknown> & {
    svgContent: string | null;
    width?: number;
    height?: number;
    scale?: number;
  };

  type SVGBackgroundNode = Node<SVGBackgroundData>;

  let { data }: NodeProps<SVGBackgroundNode> = $props();

  const scale = $derived(data.scale ?? 1);

  // Extract viewBox dimensions from SVG if available
  const svgDimensions = $derived.by(() => {
    if (!data.svgContent) return { width: 800 * scale, height: 600 * scale };

    // Try to extract viewBox
    const viewBoxMatch = data.svgContent.match(/viewBox=["']([^"']+)["']/);
    if (viewBoxMatch) {
      const parts = viewBoxMatch[1].split(/\s+/).map(Number);
      if (parts.length === 4) {
        return { width: parts[2] * scale, height: parts[3] * scale };
      }
    }

    // Try to extract width/height attributes
    const widthMatch = data.svgContent.match(/width=["'](\d+)/);
    const heightMatch = data.svgContent.match(/height=["'](\d+)/);

    return {
      width: (widthMatch ? parseInt(widthMatch[1]) : (data.width ?? 800)) * scale,
      height: (heightMatch ? parseInt(heightMatch[1]) : (data.height ?? 600)) * scale,
    };
  });
</script>

<div
  class="svg-background-node"
  style="width: {svgDimensions.width}px; height: {svgDimensions.height}px;"
>
  {#if data.svgContent}
    <div class="svg-container">
      {@html data.svgContent}
    </div>
  {:else}
    <div class="placeholder">
      <div class="placeholder-content">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          class="placeholder-icon"
        >
          <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 21h19.5m-18-18v18m10.5-18v18m6-13.5V21M6.75 6.75h.75m-.75 3h.75m-.75 3h.75m3-6h.75m-.75 3h.75m-.75 3h.75M6.75 21v-3.375c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21M3 3h12m-.75 4.5H21m-3.75 3.75h.008v.008h-.008v-.008Zm0 3h.008v.008h-.008v-.008Zm0 3h.008v.008h-.008v-.008Z" />
        </svg>
        <span class="placeholder-text">Upload a floor plan</span>
        <span class="placeholder-hint">Drag & drop an SVG file or use the upload button</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .svg-background-node {
    pointer-events: none;
    user-select: none;
    position: relative;
  }

  .svg-container {
    width: 100%;
    height: 100%;
    opacity: 0.85;
  }

  .svg-container :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  /* Placeholder styling */
  .placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: repeating-linear-gradient(
      45deg,
      var(--color-floor-plan-placeholder-stripe, #f3f4f6) 0px,
      var(--color-floor-plan-placeholder-stripe, #f3f4f6) 10px,
      var(--color-floor-plan-placeholder-bg, #e5e7eb) 10px,
      var(--color-floor-plan-placeholder-bg, #e5e7eb) 20px
    );
    border: 2px dashed var(--color-floor-plan-placeholder-border, #9ca3af);
    border-radius: 12px;
    opacity: 0.6;
  }

  .placeholder-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 2rem;
    background: var(--color-card-bg, white);
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .placeholder-icon {
    width: 48px;
    height: 48px;
    color: var(--color-floor-plan-placeholder-icon, #6b7280);
  }

  .placeholder-text {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-floor-plan-placeholder-text, #374151);
  }

  .placeholder-hint {
    font-size: 0.75rem;
    color: var(--color-floor-plan-placeholder-hint, #9ca3af);
    text-align: center;
    max-width: 200px;
  }
</style>
