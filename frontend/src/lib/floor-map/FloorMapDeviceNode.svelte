<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";
  import type { Node, NodeProps } from "@xyflow/svelte";
  import FloorMapCard from "./FloorMapCard.svelte";
  import type { NetworkDevice, DeviceState } from "../types/devices";
  import type { SensorReading } from "../api";

  type FloorMapNodeData = Record<string, unknown> & {
    device: NetworkDevice;
    deviceState?: DeviceState | null;
    latestReading?: SensorReading | null;
    switchState?: boolean;
    onDeviceClick?: (deviceId: string) => void;
    onSwitchToggle?: (deviceId: string, newState: boolean) => void;
    onTurboToggle?: (deviceId: string, newState: boolean) => void;
  };

  type FloorMapNode = Node<FloorMapNodeData>;

  let { data }: NodeProps<FloorMapNode> = $props();

  const isCoordinator = $derived(
    data.device.capabilities.some((cap) => cap.type === "coordinator") ||
      !data.device.parent_device_id,
  );
  const isBridge = $derived(data.device.is_bridge);

  function handleClick() {
    data.onDeviceClick?.(data.device.id);
  }
</script>

<div class="floor-map-node">
  {#if !isCoordinator}
    <Handle type="target" position={Position.Top} class="handle" />
  {/if}

  <FloorMapCard
    device={data.device}
    deviceState={data.deviceState ?? null}
    latestReading={data.latestReading ?? null}
    switchState={data.switchState ?? false}
    onclick={handleClick}
    onSwitchToggle={data.onSwitchToggle}
    onTurboToggle={data.onTurboToggle}
  />

  {#if isBridge || isCoordinator}
    <Handle type="source" position={Position.Bottom} class="handle" />
  {/if}
</div>

<style>
  .floor-map-node {
    position: relative;
  }

  .floor-map-node :global(.handle) {
    width: 8px;
    height: 8px;
    background: #64748b;
    border: 2px solid white;
    border-radius: 50%;
  }
</style>
