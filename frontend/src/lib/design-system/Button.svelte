<script lang="ts">
  import type { Snippet } from "svelte";

  type ButtonVariant = "primary" | "secondary" | "ghost";
  type ButtonSize = "sm" | "md";

  let {
    children,
    variant = "secondary",
    size = "md",
    block = false,
    class: className,
    ...rest
  }: {
    variant?: ButtonVariant;
    size?: ButtonSize;
    block?: boolean;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<button
  {...rest}
  class={`button ${variant} ${size} ${block ? "block" : ""} ${className ?? ""}`}
>
  {@render children?.()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-4);
    font-weight: 600;
    border: 1px solid transparent;
    transition: var(--transition-base);
    background: var(--color-card-bg);
    color: var(--color-text);
  }

  .button.block {
    width: 100%;
  }

  .button.sm {
    padding: var(--space-1) var(--space-3);
    font-size: 0.875rem;
  }

  .button.md {
    font-size: 0.95rem;
  }

  .button.primary {
    background: var(--color-card-active);
    color: #ffffff;
    border-color: var(--color-card-active);
  }

  .button.primary:hover {
    box-shadow: var(--shadow-sm);
  }

  .button.secondary {
    background: var(--color-card-bg);
    border-color: var(--color-card-border);
  }

  .button.secondary:hover {
    border-color: var(--color-card-border-hover);
    box-shadow: var(--shadow-xs);
  }

  .button.ghost {
    background: transparent;
    color: var(--color-text-muted);
  }

  .button.ghost:hover {
    color: var(--color-text);
  }
</style>
