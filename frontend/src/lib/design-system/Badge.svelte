<script lang="ts">
  import type { Snippet } from "svelte";

  type BadgeVariant = "neutral" | "good" | "medium" | "low" | "info";
  type BadgeSize = "sm" | "mini";

  let {
    children,
    variant = "neutral",
    size = "sm",
    class: className,
    ...rest
  }: {
    variant?: BadgeVariant;
    size?: BadgeSize;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<span
  {...rest}
  class={`badge ${variant} ${size} ${className ?? ""}`}
>
  {@render children?.()}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    font-size: 0.75rem;
    font-weight: 600;
    border: 1px solid var(--color-badge-neutral-border);
    background: var(--color-badge-neutral-bg);
    color: var(--color-badge-neutral-text);
  }

  .badge :global(svg) {
    width: 14px;
    height: 14px;
  }

  .badge.mini {
    gap: 2px;
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    font-size: 0.55rem;
    border: none;
  }

  .badge.mini :global(svg) {
    width: 8px;
    height: 8px;
  }

  .badge.good {
    background: var(--color-badge-success-bg);
    color: var(--color-badge-success-text);
    border-color: var(--color-badge-success-border);
  }

  .badge.medium {
    background: var(--color-badge-warning-bg);
    color: var(--color-badge-warning-text);
    border-color: var(--color-badge-warning-border);
  }

  .badge.low {
    background: var(--color-badge-danger-bg);
    color: var(--color-badge-danger-text);
    border-color: var(--color-badge-danger-border);
  }

  .badge.info {
    background: var(--color-card-active-bg);
    color: var(--color-card-active);
    border-color: rgba(59, 130, 246, 0.3);
  }
</style>
