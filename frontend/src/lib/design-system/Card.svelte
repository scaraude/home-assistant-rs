<script lang="ts">
  import type { Snippet } from "svelte";

  type CardSize = "sm" | "md" | "lg";

  let {
    children,
    active = false,
    interactive = false,
    size = "md",
    class: className,
    ...rest
  }: {
    active?: boolean;
    interactive?: boolean;
    size?: CardSize;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<div
  {...rest}
  class={`card size-${size} ${interactive ? "interactive" : ""} ${active ? "active" : ""} ${className ?? ""}`}
>
  {@render children?.()}
</div>

<style>
  .card {
    position: relative;
    background: var(--color-card-bg);
    border: 2px solid var(--color-card-border);
    border-radius: var(--radius-md);
    padding: var(--space-3_5);
    transition: var(--transition-fast);
    box-shadow: var(--shadow-xs);
  }

  .card.size-sm {
    padding: var(--space-3);
  }

  .card.size-md {
    padding: var(--space-3_5);
  }

  .card.size-lg {
    padding: var(--space-5);
  }

  .card.interactive {
    cursor: pointer;
    user-select: none;
  }

  .card.interactive:hover {
    border-color: var(--color-card-border-hover);
    box-shadow: var(--shadow-sm);
  }

  .card.active {
    border-color: var(--color-card-active);
    background: var(--color-card-active-bg);
    box-shadow: var(--shadow-active);
  }
</style>
