<script lang="ts">
  import {
    SvelteFlow,
    Controls,
    Background,
    BackgroundVariant,
    type Node,
    type Edge,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { networkTopologyStore } from "../stores/networkTopology";
  import DeviceNodeComponent from "./DeviceNode.svelte";

  import type { Device } from "../stores/networkTopology";

  type DeviceState = {
    link_quality: number | null;
    battery_level: number | null;
  };

  interface Props {
    deviceStates?: Map<string, DeviceState>;
  }

  type DeviceNodeData = Record<string, unknown> & {
    device: Device;
    deviceState?: DeviceState;
  };

  type DeviceNodeType = Node<DeviceNodeData>;
  type NodeDragStopEvent = {
    targetNode: DeviceNodeType | null;
    nodes: DeviceNodeType[];
    event: MouseEvent | TouchEvent;
  };

  let { deviceStates = new Map() }: Props = $props();

  const nodeTypes = {
    device: DeviceNodeComponent,
  };

  const nodes = $derived.by((): DeviceNodeType[] => {
    const topology = $networkTopologyStore.topology;
    if (!topology) return [];

    const positions = $networkTopologyStore.positions;
    return topology.devices.map((device, index) => {
      const savedPos = positions[device.id];
      const pos =
        savedPos ||
        calculateInitialPosition(device, index, topology.devices.length);

      return {
        id: device.id,
        type: "device",
        position: pos,
        data: {
          device,
          deviceState: deviceStates.get(device.id),
        },
        draggable: true,
      };
    });
  });

  const edges = $derived.by((): Edge[] => {
    const topology = $networkTopologyStore.topology;
    if (!topology) return [];

    return topology.edges.map((edge) => {
      const linkQuality = edge.link_quality;
      const color = getLinkQualityColor(linkQuality);

      return {
        id: `${edge.source_id}-${edge.target_id}`,
        source: edge.source_id,
        target: edge.target_id,
        animated: false,
        style: `stroke: ${color}; stroke-width: 2px;`,
        label: linkQuality != null ? `LQI: ${linkQuality}` : undefined,
      };
    });
  });

  function calculateInitialPosition(
    device: Device,
    index: number,
    total: number
  ): { x: number; y: number } {
    // Place coordinator at top center
    if (!device.parent_device_id) {
      return { x: 400, y: 50 };
    }

    // Simple circular layout for other devices
    const angle = (index / total) * 2 * Math.PI;
    const radius = 300;
    return {
      x: 400 + Math.cos(angle) * radius,
      y: 250 + Math.sin(angle) * radius,
    };
  }

  function getLinkQualityColor(lqi: number | null | undefined): string {
    if (lqi == null) return "#9ca3af";
    if (lqi > 100) return "#22c55e";
    if (lqi >= 50) return "#eab308";
    return "#ef4444";
  }

  function handleNodeDragStop({ targetNode }: NodeDragStopEvent) {
    if (!targetNode) return;
    networkTopologyStore.updateNodePosition(
      targetNode.id,
      targetNode.position.x,
      targetNode.position.y
    );
  }
</script>

<div class="network-graph">
  <SvelteFlow
    {nodes}
    {edges}
    {nodeTypes}
    fitView
    onnodedragstop={handleNodeDragStop}
  >
    <Background variant={BackgroundVariant.Dots} />
    <Controls />
  </SvelteFlow>
</div>

<style>
  .network-graph {
    width: 100%;
    height: 600px;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
  }

  :global(.svelte-flow__edge-label) {
    font-size: 10px;
    background: white;
    padding: 2px 4px;
    border-radius: 3px;
  }
</style>
