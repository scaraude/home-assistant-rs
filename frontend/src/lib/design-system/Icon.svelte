<script lang="ts">
  import type { SVGAttributes } from "svelte/elements";
  import { icons } from "../icons";

  type IconName = keyof typeof icons;

  let {
    name,
    title,
    class: className,
    size,
    ...rest
  }: {
    name: IconName;
    title?: string;
    size?: number | string;
    class?: string;
  } & SVGAttributes<SVGSVGElement> = $props();
</script>

<svg
  {...rest}
  class={className}
  viewBox="0 0 24 24"
  fill="currentColor"
  aria-hidden={title ? undefined : true}
  role={title ? "img" : undefined}
  width={size}
  height={size}
>
  {#if title}
    <title>{title}</title>
  {/if}
  <path d={icons[name]} />
</svg>
