<script lang="ts">
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import {
    SvelteFlow,
    Controls,
    Background,
    BackgroundVariant,
    type Node,
    type Edge,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { fetchDeviceStates, fetchNetworkTopology } from "../api";
  import { fetchFloorPlan, uploadFloorPlan } from "../api/floor-map";
  import { dataCache } from "../stores/dataCache";
  import { floorMapStore } from "../stores/floorMapStore";
  import { networkTopologyStore } from "../stores/networkTopology";
  import type { NetworkDevice } from "../types/devices";
  import type { SensorReading } from "../api";
  import FloorMapDeviceNode from "./FloorMapDeviceNode.svelte";
  import SVGBackgroundNode from "./SVGBackgroundNode.svelte";

  type FloorMapDeviceNodeData = {
    kind: "device";
    device: NetworkDevice;
    deviceState?: import("../types/devices").DeviceState | null;
    latestReading?: SensorReading | null;
    switchState?: boolean;
    onDeviceClick?: (deviceId: string) => void;
  };

  type FloorMapBackgroundNodeData = {
    kind: "background";
    svgContent: string | null;
  };

  type FloorMapNodeData = FloorMapDeviceNodeData | FloorMapBackgroundNodeData;

  type DeviceNodeType = Node<FloorMapNodeData>;
  type NodeDragStopEvent = {
    targetNode: DeviceNodeType | null;
    nodes: DeviceNodeType[];
    event: MouseEvent | TouchEvent;
  };

  let floorPlanSvg = $state<string | null>(null);
  let floorPlanLoading = $state(false);
  let floorPlanError = $state<string | null>(null);
  let uploadError = $state<string | null>(null);
  let uploading = $state(false);
  let showConnections = $state(true);
  let dragActive = $state(false);
  let fileInput: HTMLInputElement | null = null;
  let mapWrapperEl: HTMLDivElement | null = $state(null);

  const nodeTypes = {
    "background-svg": SVGBackgroundNode,
    "floor-device": FloorMapDeviceNode,
  };

  onMount(() => {
    void Promise.all([
      loadTopology(),
      loadFloorPlan(),
      floorMapStore.initialize(),
    ]);
  });

  async function loadTopology() {
    networkTopologyStore.setLoading(true);
    try {
      const topology = await fetchNetworkTopology();
      networkTopologyStore.setTopology(topology);
      await loadDeviceStates();
    } catch (error) {
      console.error("Failed to load network topology:", error);
      networkTopologyStore.setError(
        error instanceof Error ? error.message : "Unknown error",
      );
    }
  }

  async function loadDeviceStates() {
    try {
      const states = await fetchDeviceStates();
      for (const state of states) {
        dataCache.updateDeviceState(state.device_id, state);
      }
    } catch (error) {
      console.error("Failed to load device states:", error);
    }
  }

  async function loadFloorPlan() {
    floorPlanLoading = true;
    floorPlanError = null;
    try {
      const floorPlan = await fetchFloorPlan();
      floorPlanSvg = floorPlan?.svg_content ?? null;
    } catch (error) {
      console.error("Failed to load floor plan:", error);
      floorPlanError =
        error instanceof Error ? error.message : "Failed to load floor plan";
    } finally {
      floorPlanLoading = false;
    }
  }

  function getSvgDimensions(svgContent: string | null): {
    width: number;
    height: number;
  } {
    if (!svgContent) return { width: 800, height: 600 };

    const viewBoxMatch = svgContent.match(/viewBox=["']([^"']+)["']/);
    if (viewBoxMatch) {
      const parts = viewBoxMatch[1].split(/\s+/).map(Number);
      if (parts.length === 4) {
        return { width: parts[2], height: parts[3] };
      }
    }

    const widthMatch = svgContent.match(/width=["'](\d+)/);
    const heightMatch = svgContent.match(/height=["'](\d+)/);

    return {
      width: widthMatch ? parseInt(widthMatch[1], 10) : 800,
      height: heightMatch ? parseInt(heightMatch[1], 10) : 600,
    };
  }

  function calculateInitialPosition(
    device: NetworkDevice,
    index: number,
    total: number,
  ): { x: number; y: number } {
    const { width, height } = getSvgDimensions(floorPlanSvg);
    const columns = Math.max(1, Math.ceil(Math.sqrt(total)));
    const spacingX = Math.max(140, Math.min(220, Math.floor(width / columns)));
    const spacingY = Math.max(120, Math.min(200, Math.floor(height / columns)));

    const isCoordinator =
      device.capabilities.some((cap) => cap.type === "coordinator") ||
      !device.parent_device_id;
    if (isCoordinator) {
      return { x: width / 2 - 60, y: 40 };
    }

    const col = index % columns;
    const row = Math.floor(index / columns);
    return {
      x: 40 + col * spacingX,
      y: 120 + row * spacingY,
    };
  }

  function getLinkQualityColor(lqi: number | null | undefined): string {
    if (lqi == null) return "#9ca3af";
    if (lqi > 100) return "#22c55e";
    if (lqi >= 50) return "#eab308";
    return "#ef4444";
  }

  const latestReadingByDevice = $derived.by(() => {
    const map = new SvelteMap<string, SensorReading>();
    for (const reading of $dataCache.sensors.readings) {
      map.set(reading.device_id, reading);
    }
    return map;
  });

  const nodes = $derived.by((): Node<FloorMapNodeData>[] => {
    const topology = $networkTopologyStore.topology;
    const baseNodes: Node<FloorMapNodeData>[] = [
      {
        id: "floorplan-background",
        type: "background-svg",
        position: { x: 0, y: 0 },
        data: { kind: "background", svgContent: floorPlanSvg },
        draggable: false,
        selectable: false,
        connectable: false,
        zIndex: 0,
      },
    ];

    if (!topology) return baseNodes;

    const positions = $floorMapStore.positions;

    const deviceNodes = topology.devices.map(
      (device, index): Node<FloorMapNodeData> => {
        const savedPosition = positions[device.id];
        const position =
          savedPosition ??
          calculateInitialPosition(device, index, topology.devices.length);
        const switchState = $dataCache.switches.byId[device.id]?.state ?? false;

        return {
          id: device.id,
          type: "floor-device",
          position,
          data: {
            kind: "device",
            device,
            deviceState: $dataCache.deviceStates[device.id] ?? null,
            latestReading: latestReadingByDevice.get(device.id) ?? null,
            switchState,
            onDeviceClick: handleDeviceClick,
          },
          draggable: true,
          zIndex: 2,
        };
      },
    );

    return [...baseNodes, ...deviceNodes];
  });

  const edges = $derived.by((): Edge[] => {
    if (!showConnections) return [];

    const topology = $networkTopologyStore.topology;
    if (!topology) return [];

    return topology.edges.map((edge) => {
      const color = getLinkQualityColor(edge.link_quality);
      return {
        id: `${edge.source_id}-${edge.target_id}`,
        source: edge.source_id,
        target: edge.target_id,
        animated: false,
        style: `stroke: ${color}; stroke-width: 2px;`,
      };
    });
  });

  function handleNodeDragStop({ targetNode }: NodeDragStopEvent) {
    if (!targetNode || targetNode.id === "floorplan-background") return;
    floorMapStore.updatePosition(
      targetNode.id,
      targetNode.position.x,
      targetNode.position.y,
    );
  }

  const debugSnapshot = $derived.by(() => ({
    hasTopology: Boolean($networkTopologyStore.topology),
    devices: $networkTopologyStore.topology?.devices.length ?? 0,
    edges: $networkTopologyStore.topology?.edges.length ?? 0,
    nodes: nodes.length,
    connectionsOn: showConnections,
    wrapperSize: mapWrapperEl
      ? `${Math.round(mapWrapperEl.clientWidth)}x${Math.round(mapWrapperEl.clientHeight)}`
      : "unset",
    svgLoaded: Boolean(floorPlanSvg),
  }));

  function handleDeviceClick(_deviceId: string) {
    // Placeholder for future device detail routing.
  }

  function openFilePicker() {
    fileInput?.click();
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    dragActive = true;
  }

  function handleDragLeave(event: DragEvent) {
    if (event.currentTarget === event.target) {
      dragActive = false;
    }
  }

  async function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    const file = event.dataTransfer?.files?.[0];
    if (file) {
      await handleUpload(file);
    }
  }

  async function handleFileChange(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) {
      await handleUpload(file);
    }
    if (input) {
      input.value = "";
    }
  }

  async function handleUpload(file: File) {
    uploadError = null;
    if (
      !file.type.includes("svg") &&
      !file.name.toLowerCase().endsWith(".svg")
    ) {
      uploadError = "Please upload a valid SVG file.";
      return;
    }

    uploading = true;
    try {
      const svgContent = await file.text();
      if (!svgContent.includes("<svg")) {
        throw new Error("Invalid SVG content");
      }
      await uploadFloorPlan(svgContent);
      floorPlanSvg = svgContent;
    } catch (error) {
      console.error("Failed to upload floor plan:", error);
      uploadError =
        error instanceof Error ? error.message : "Failed to upload floor plan";
    } finally {
      uploading = false;
    }
  }
</script>

<div
  class="floor-map-view"
  ondragover={handleDragOver}
  ondragenter={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  role="region"
  aria-label="Floor map"
>
  <div class="header">
    <div class="title">
      <h2>Floor Map</h2>
      <p>Drag devices to position them on your floor plan.</p>
    </div>

    <div class="toolbar">
      <label class="toggle">
        <input type="checkbox" bind:checked={showConnections} />
        <span>Network connections</span>
      </label>

      <button
        class="btn btn-secondary"
        onclick={openFilePicker}
        disabled={uploading}
      >
        {#if uploading}
          Uploading...
        {:else}
          Upload SVG
        {/if}
      </button>
      <input
        type="file"
        accept="image/svg+xml"
        bind:this={fileInput}
        onchange={handleFileChange}
      />
    </div>
  </div>

  {#if floorPlanError}
    <div class="banner error">❌ {floorPlanError}</div>
  {/if}
  {#if uploadError}
    <div class="banner error">❌ {uploadError}</div>
  {/if}
  {#if $floorMapStore.error}
    <div class="banner warning">⚠️ {$floorMapStore.error}</div>
  {/if}

  {#if $networkTopologyStore.loading || floorPlanLoading}
    <div class="loading">
      <div class="spinner"></div>
      <p>Loading floor map...</p>
    </div>
  {:else if $networkTopologyStore.error}
    <div class="error">
      <p>❌ Error: {$networkTopologyStore.error}</p>
      <button class="btn btn-primary" onclick={loadTopology}>Retry</button>
    </div>
  {:else}
    <div
      class="map-wrapper"
      class:drag-active={dragActive}
      bind:this={mapWrapperEl}
    >
      <SvelteFlow
        {nodes}
        {edges}
        {nodeTypes}
        fitView
        minZoom={0.2}
        maxZoom={2}
        onnodedragstop={handleNodeDragStop}
        style="width: 100%; height: 100%;"
      >
        <Background variant={BackgroundVariant.Lines} gap={32} size={1} />
        <Controls />
      </SvelteFlow>

      {#if dragActive}
        <div class="drop-overlay">
          <div class="drop-card">
            <strong>Drop your SVG to upload</strong>
            <span>Release to replace the current floor plan.</span>
          </div>
        </div>
      {/if}
    </div>

    <div class="status-bar">
      <div class="status-item">
        <span class="status-label">Devices</span>
        <span class="status-value">
          {$networkTopologyStore.topology?.devices.length ?? 0}
        </span>
      </div>
      <div class="status-item">
        <span class="status-label">Connections</span>
        <span class="status-value">
          {$networkTopologyStore.topology?.edges.length ?? 0}
        </span>
      </div>
      <div class="status-item">
        <span class="status-label">Sync</span>
        <span
          class="status-value"
          class:syncing={$floorMapStore.syncStatus === "syncing"}
        >
          {$floorMapStore.syncStatus === "syncing" ? "Saving..." : "Up to date"}
        </span>
      </div>
    </div>

    <div class="debug-bar">
      <span>topology: {debugSnapshot.hasTopology ? "yes" : "no"}</span>
      <span>devices: {debugSnapshot.devices}</span>
      <span>edges: {debugSnapshot.edges}</span>
      <span>nodes: {debugSnapshot.nodes}</span>
      <span>connections: {debugSnapshot.connectionsOn ? "on" : "off"}</span>
      <span>map: {debugSnapshot.wrapperSize}</span>
      <span>svg: {debugSnapshot.svgLoaded ? "yes" : "no"}</span>
    </div>
  {/if}
</div>

<style>
  .floor-map-view {
    padding: 20px;
    max-width: 1500px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .title h2 {
    margin: 0;
    font-size: 24px;
  }

  .title p {
    margin: 4px 0 0;
    color: #6b7280;
    font-size: 14px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: #374151;
  }

  .toggle input {
    width: 16px;
    height: 16px;
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-primary {
    background-color: #3b82f6;
    color: white;
  }

  .btn-secondary {
    background-color: #6b7280;
    color: white;
  }

  input[type="file"] {
    display: none;
  }

  .banner {
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 14px;
  }

  .banner.error {
    background: #fee2e2;
    color: #b91c1c;
  }

  .banner.warning {
    background: #fef3c7;
    color: #92400e;
  }

  .map-wrapper {
    position: relative;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
    min-height: 520px;
    height: 70vh;
    max-height: 720px;
    background: #f9fafb;
  }

  .map-wrapper :global(.svelte-flow) {
    width: 100%;
    height: 100%;
  }

  .map-wrapper.drag-active {
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    background: rgba(59, 130, 246, 0.08);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .drop-card {
    background: white;
    border-radius: 12px;
    padding: 20px 24px;
    box-shadow: 0 12px 24px rgba(15, 23, 42, 0.15);
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-align: center;
    color: #1f2937;
  }

  .loading,
  .error {
    text-align: center;
    padding: 60px 20px;
  }

  .loading .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid rgba(59, 130, 246, 0.2);
    border-top-color: #3b82f6;
    border-radius: 50%;
    margin: 0 auto 12px;
    animation: spin 0.8s linear infinite;
  }

  .status-bar {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
    padding: 12px 16px;
    background: #f9fafb;
    border-radius: 10px;
    border: 1px solid #e5e7eb;
  }

  .debug-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px dashed #d1d5db;
    font-size: 12px;
    color: #6b7280;
  }

  .status-item {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .status-label {
    font-size: 12px;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-value {
    font-size: 16px;
    font-weight: 600;
    color: #111827;
  }

  .status-value.syncing {
    color: #2563eb;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 800px) {
    .floor-map-view {
      padding: 16px;
    }

    .map-wrapper {
      min-height: 420px;
    }

    .status-bar {
      gap: 12px;
    }
  }
</style>
