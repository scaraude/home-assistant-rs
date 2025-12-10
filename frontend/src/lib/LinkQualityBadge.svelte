<script lang="ts">
  let { linkQuality }: { linkQuality: number } = $props();
</script>

<div
  class="link-quality-badge"
  class:good={linkQuality >= 100}
  class:medium={linkQuality >= 50 && linkQuality < 100}
  class:poor={linkQuality < 50}
>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="currentColor"
  >
    <path
      d="M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3c-1.65-1.66-4.34-1.66-6 0zm-4-4l2 2c2.76-2.76 7.24-2.76 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"
    />
  </svg>
  {linkQuality}
</div>

<style>
  .link-quality-badge {
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

  .link-quality-badge svg {
    width: 14px;
    height: 14px;
  }

  .link-quality-badge.good {
    background: rgba(34, 197, 94, 0.1);
    color: #16a34a;
    border-color: rgba(34, 197, 94, 0.3);
  }

  .link-quality-badge.medium {
    background: rgba(251, 191, 36, 0.1);
    color: #d97706;
    border-color: rgba(251, 191, 36, 0.3);
  }

  .link-quality-badge.poor {
    background: rgba(239, 68, 68, 0.1);
    color: #dc2626;
    border-color: rgba(239, 68, 68, 0.3);
  }
</style>
