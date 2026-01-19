<script lang="ts">
  import Badge from "../design-system/Badge.svelte";

  type BadgeType = "battery" | "signal";

  let { type, value, mini = false }: { type: BadgeType; value: number; mini?: boolean } =
    $props();

  type BadgeVariant = "good" | "medium" | "low";

  const thresholds = {
    battery: { good: 75, medium: 25 },
    signal: { good: 100, medium: 50 },
  };

  function levelFor(type: BadgeType, value: number): BadgeVariant {
    return value >= thresholds[type].good
      ? "good"
      : value >= thresholds[type].medium
        ? "medium"
        : "low";
  }

  let level = $derived(levelFor(type, value));

  let label = $derived(type === "battery" ? "Battery" : "Link quality");
  let titleText = $derived(
    `${label}: ${value}${type === "battery" ? "%" : ""}`
  );
</script>

<Badge variant={level} size={mini ? "mini" : "sm"} title={titleText}>
  {#if type === "battery"}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
      <path
        d="M15.67 4H14V2h-4v2H8.33C7.6 4 7 4.6 7 5.33v15.33C7 21.4 7.6 22 8.33 22h7.33c.74 0 1.34-.6 1.34-1.33V5.33C17 4.6 16.4 4 15.67 4z"
      />
    </svg>
  {:else}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
      <path
        d="M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3c-1.65-1.66-4.34-1.66-6 0zm-4-4l2 2c2.76-2.76 7.24-2.76 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"
      />
    </svg>
  {/if}
  {value}{type === "battery" ? "%" : ""}
</Badge>
