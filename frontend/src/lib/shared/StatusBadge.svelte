<script lang="ts">
  import Badge from "../design-system/Badge.svelte";
  import Icon from "../design-system/Icon.svelte";
  import { getLqiBadgeVariant } from "../utils/badge";

  type BadgeType = "battery" | "signal";

  let { type, value, mini = false }: { type: BadgeType; value: number; mini?: boolean } =
    $props();

  type BadgeVariant = "good" | "medium" | "low";

  const thresholds = {
    battery: { good: 75, medium: 25 },
  };

  function levelFor(type: BadgeType, value: number): BadgeVariant {
    if (type === "signal") return getLqiBadgeVariant(value);
    return value >= thresholds.battery.good
      ? "good"
      : value >= thresholds.battery.medium
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
    <Icon name="battery" />
  {:else}
    <Icon name="signal" />
  {/if}
  {value}{type === "battery" ? "%" : ""}
</Badge>
