<script lang="ts">
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
  };

  type FloorMapNode = Node<FloorMapNodeData>;

  let { data }: NodeProps<FloorMapNode> = $props();

  function handleClick() {
    data.onDeviceClick?.(data.device.id);
  }
</script>

<FloorMapCard
  device={data.device}
  deviceState={data.deviceState ?? null}
  latestReading={data.latestReading ?? null}
  switchState={data.switchState ?? false}
  onclick={handleClick}
/>
