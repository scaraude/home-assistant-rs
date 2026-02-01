<script lang="ts">
  import type { Snippet } from "svelte";

  type GaugeSize = "sm" | "md" | "lg";
  type GaugeTone = "neutral" | "cool" | "warm" | "good" | "info";

  const CIRCUMFERENCE = 339.292;

  let {
    value = null,
    min = 0,
    max = 100,
    label,
    unit,
    size = "md",
    tone = "neutral",
    center,
    precision,
    format,
    ariaLabel,
    class: className,
    ...rest
  }: {
    value?: number | null;
    min?: number;
    max?: number;
    label?: string;
    unit?: string;
    size?: GaugeSize;
    tone?: GaugeTone;
    center?: Snippet;
    precision?: number;
    format?: (value: number) => string;
    ariaLabel?: string;
    class?: string;
    [key: string]: unknown;
  } = $props();

  const progress = $derived.by(() => {
    if (value === null || Number.isNaN(value)) {
      return 0;
    }

    const safeMin = Math.min(min, max);
    const safeMax = Math.max(min, max);
    if (safeMax === safeMin) {
      return 0;
    }

    const normalized = (value - safeMin) / (safeMax - safeMin);
    return Math.min(Math.max(normalized, 0), 1);
  });

  const displayValue = $derived.by(() => {
    if (value === null || Number.isNaN(value)) {
      return "--";
    }

    if (format) {
      return format(value);
    }

    if (precision !== undefined) {
      return value.toFixed(precision);
    }

    return String(value);
  });

  const computedAriaLabel = $derived.by(() => {
    if (ariaLabel) {
      return ariaLabel;
    }

    const suffix = unit ? ` ${unit}` : "";
    const prefix = label ? `${label}: ` : "";
    return `${prefix}${displayValue}${suffix}`;
  });
</script>

<div
  {...rest}
  class={`gauge size-${size} tone-${tone} ${className ?? ""}`}
  role="img"
  aria-label={computedAriaLabel}
  style={`--progress: ${progress}; --circumference: ${CIRCUMFERENCE};`}
>
  <div class="gauge-ring">
    <svg viewBox="0 0 120 120" aria-hidden="true">
      <circle cx="60" cy="60" r="54" class="gauge-track" />
      <circle cx="60" cy="60" r="54" class="gauge-meter" />
    </svg>
    <div class="gauge-center">
      {#if center}
        {@render center()}
      {:else}
        <span class="gauge-value">{displayValue}</span>
        {#if unit}
          <span class="gauge-unit">{unit}</span>
        {/if}
      {/if}
    </div>
  </div>
  {#if label}
    <div class="gauge-label">{label}</div>
  {/if}
</div>

<style>
  .gauge {
    --gauge-track: var(--color-card-border);
    --gauge-fill: var(--color-card-active);
    --gauge-text: var(--color-text);
    --gauge-subtle: var(--color-text-muted);
    --gauge-shadow: none;
    --gauge-size: 120px;
    --gauge-stroke: 8;
    --gauge-value-size: 1.75rem;
    --gauge-unit-size: 0.85rem;
    --gauge-label-size: 0.7rem;

    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }

  .gauge.size-sm {
    --gauge-size: 96px;
    --gauge-stroke: 7;
    --gauge-value-size: 1.35rem;
    --gauge-unit-size: 0.75rem;
    --gauge-label-size: 0.65rem;
  }

  .gauge.size-lg {
    --gauge-size: 160px;
    --gauge-stroke: 10;
    --gauge-value-size: 2.4rem;
    --gauge-unit-size: 1rem;
    --gauge-label-size: 0.75rem;
  }

  .gauge.tone-neutral {
    --gauge-fill: #94a3b8;
    --gauge-shadow: none;
  }

  .gauge.tone-cool {
    --gauge-fill: #3b82f6;
    --gauge-shadow: 0 0 12px rgba(59, 130, 246, 0.35);
  }

  .gauge.tone-warm {
    --gauge-fill: #f59e0b;
    --gauge-shadow: 0 0 12px rgba(245, 158, 11, 0.35);
  }

  .gauge.tone-good {
    --gauge-fill: #22c55e;
    --gauge-shadow: 0 0 12px rgba(34, 197, 94, 0.35);
  }

  .gauge.tone-info {
    --gauge-fill: #06b6d4;
    --gauge-shadow: 0 0 12px rgba(6, 182, 212, 0.35);
  }

  .gauge-ring {
    position: relative;
    width: var(--gauge-size);
    height: var(--gauge-size);
  }

  .gauge-ring svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }

  .gauge-track {
    fill: none;
    stroke: var(--gauge-track);
    stroke-width: var(--gauge-stroke);
  }

  .gauge-meter {
    fill: none;
    stroke: var(--gauge-fill);
    stroke-width: var(--gauge-stroke);
    stroke-linecap: round;
    stroke-dasharray: var(--circumference);
    stroke-dashoffset: calc(var(--circumference) * (1 - var(--progress)));
    transition: stroke-dashoffset var(--transition-base);
    filter: drop-shadow(var(--gauge-shadow));
  }

  .gauge-center {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }

  .gauge-value {
    font-size: var(--gauge-value-size);
    font-weight: 700;
    color: var(--gauge-text);
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }

  .gauge-unit {
    font-size: var(--gauge-unit-size);
    font-weight: 600;
    color: var(--gauge-subtle);
    margin-top: 8px;
  }

  .gauge-label {
    font-size: var(--gauge-label-size);
    font-weight: 600;
    color: var(--gauge-subtle);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  @media (prefers-reduced-motion: reduce) {
    .gauge-meter {
      transition: none;
    }
  }
</style>
