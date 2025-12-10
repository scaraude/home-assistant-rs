<script lang="ts">
  let { battery }: { battery: number } = $props();
</script>

<div
  class="battery-badge"
  class:good={battery >= 75}
  class:medium={battery >= 25 && battery < 75}
  class:low={battery < 25}
>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="currentColor"
  >
    <path
      d="M15.67 4H14V2h-4v2H8.33C7.6 4 7 4.6 7 5.33v15.33C7 21.4 7.6 22 8.33 22h7.33c.74 0 1.34-.6 1.34-1.33V5.33C17 4.6 16.4 4 15.67 4z"
    />
  </svg>
  {battery}%
</div>

<style>
  .battery-badge {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 600;
    background: rgba(107, 114, 128, 0.1);
    color: #6b7280;
    border: 1px solid rgba(107, 114, 128, 0.2);
  }

  .battery-badge svg {
    width: 14px;
    height: 14px;
  }

  .battery-badge.good {
    background: rgba(34, 197, 94, 0.1);
    color: #16a34a;
    border-color: rgba(34, 197, 94, 0.3);
  }

  .battery-badge.medium {
    background: rgba(251, 191, 36, 0.1);
    color: #d97706;
    border-color: rgba(251, 191, 36, 0.3);
  }

  .battery-badge.low {
    background: rgba(239, 68, 68, 0.1);
    color: #dc2626;
    border-color: rgba(239, 68, 68, 0.3);
  }
</style>
