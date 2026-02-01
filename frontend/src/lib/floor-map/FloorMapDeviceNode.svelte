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
    width: 10px;
    height: 10px;
    background: linear-gradient(135deg, #94a3b8 0%, #64748b 100%);
    border: 2px solid white;
    border-radius: 50%;
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.1),
      inset 0 1px 0 rgba(255, 255, 255, 0.3);
    transition: all 0.2s ease;
  }

  .floor-map-node:hover :global(.handle) {
    transform: scale(1.2);
    background: linear-gradient(135deg, #60a5fa 0%, #3b82f6 100%);
    box-shadow:
      0 2px 8px rgba(59, 130, 246, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.3);
  }
</style>
