<script lang="ts">
  import type { Snippet } from "svelte";

  type TextVariant = "title" | "subtitle" | "body" | "caption" | "overline";

  let {
    children,
    as = "span",
    variant = "body",
    class: className,
    ...rest
  }: {
    as?: keyof HTMLElementTagNameMap;
    variant?: TextVariant;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<svelte:element
  this={as}
  {...rest}
  class={`text ${variant} ${className ?? ""}`}
>
  {@render children?.()}
</svelte:element>

<style>
  .text {
    margin: 0;
    color: var(--color-text);
  }

  .text.title {
    font-size: 1.1rem;
    font-weight: 700;
  }

  .text.subtitle {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--color-text-muted);
  }

  .text.body {
    font-size: 0.9rem;
    color: var(--color-text);
  }

  .text.caption {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .text.overline {
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-text-subtle);
  }
</style>
