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
    scale?: number;
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
  let backgroundScale = $state(1.5);
  let dragActive = $state(false);
  let fileInput = $state<HTMLInputElement | null>(null);

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
        data: {
          kind: "background",
          svgContent: floorPlanSvg,
          scale: backgroundScale,
        },
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

  const connectionCount = $derived(
    $networkTopologyStore.topology?.edges.length ?? 0,
  );
  const deviceCount = $derived(
    $networkTopologyStore.topology?.devices.length ?? 0,
  );

  function handleNodeDragStop({ targetNode }: NodeDragStopEvent) {
    if (!targetNode || targetNode.id === "floorplan-background") return;
    floorMapStore.updatePosition(
      targetNode.id,
      targetNode.position.x,
      targetNode.position.y,
    );
  }

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
    <div class="map-wrapper" class:drag-active={dragActive}>
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

      <div class="map-overlay top-right">
        <label class="switch">
          <input type="checkbox" bind:checked={showConnections} />
          <span class="switch-track" aria-hidden="true">
            <span class="switch-thumb"></span>
          </span>
          <span class="switch-label">Connections {connectionCount}</span>
        </label>

        <div class="scale-control">
          <label class="scale-label" for="bg-scale"
            >Scale {backgroundScale.toFixed(1)}x</label
          >
          <input
            type="range"
            id="bg-scale"
            min="0.5"
            max="4"
            step="0.1"
            bind:value={backgroundScale}
            class="scale-slider"
          />
        </div>

        <button
          class="btn btn-secondary overlay-button"
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

      <div class="map-overlay bottom-right">
        <div class="status-pill">
          <span class="status-label">Devices</span>
          <span class="status-value">{deviceCount}</span>
        </div>
        <div class="status-pill">
          <span class="status-label">Connections</span>
          <span class="status-value">{connectionCount}</span>
        </div>
        <div class="status-pill">
          <span class="status-label">Sync</span>
          <span
            class="status-value"
            class:syncing={$floorMapStore.syncStatus === "syncing"}
          >
            {$floorMapStore.syncStatus === "syncing"
              ? "Saving..."
              : "Up to date"}
          </span>
        </div>
      </div>

      {#if floorPlanError || uploadError || $floorMapStore.error}
        <div class="map-notices">
          {#if floorPlanError}
            <div class="banner error">❌ {floorPlanError}</div>
          {/if}
          {#if uploadError}
            <div class="banner error">❌ {uploadError}</div>
          {/if}
          {#if $floorMapStore.error}
            <div class="banner warning">⚠️ {$floorMapStore.error}</div>
          {/if}
        </div>
      {/if}

      {#if dragActive}
        <div class="drop-overlay">
          <div class="drop-card">
            <strong>Drop your SVG to upload</strong>
            <span>Release to replace the current floor plan.</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .floor-map-view {
    height: calc(100vh - var(--app-header-height) - var(--app-nav-height));
    width: 100vw;
    margin: 0 calc(50% - 50vw);
    padding: 0;
    display: flex;
    flex-direction: column;
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
    border-radius: 0;
    overflow: hidden;
    min-height: 0;
    height: 100%;
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

  .map-overlay {
    position: absolute;
    z-index: 5;
    display: flex;
    gap: 10px;
    align-items: center;
    background: rgba(255, 255, 255, 0.92);
    padding: 10px 12px;
    border-radius: 10px;
    box-shadow: 0 8px 18px rgba(15, 23, 42, 0.12);
    border: 1px solid rgba(148, 163, 184, 0.4);
    backdrop-filter: blur(10px);
  }

  .map-overlay.top-right {
    top: 16px;
    right: 16px;
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .map-overlay.bottom-right {
    bottom: 16px;
    right: 16px;
    gap: 12px;
    flex-direction: column;
    align-items: stretch;
  }

  .overlay-button {
    width: 100%;
  }

  .switch {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 600;
    color: #1f2937;
    cursor: pointer;
  }

  .switch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .switch-track {
    width: 42px;
    height: 22px;
    border-radius: 999px;
    background: #cbd5f5;
    display: inline-flex;
    align-items: center;
    padding: 2px;
    transition: background 0.2s ease;
  }

  .switch-thumb {
    width: 18px;
    height: 18px;
    border-radius: 999px;
    background: white;
    box-shadow: 0 2px 6px rgba(15, 23, 42, 0.25);
    transform: translateX(0);
    transition: transform 0.2s ease;
  }

  .switch input:checked + .switch-track {
    background: #2563eb;
  }

  .switch input:checked + .switch-track .switch-thumb {
    transform: translateX(20px);
  }

  .switch-label {
    white-space: nowrap;
  }

  .scale-control {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .scale-label {
    font-size: 13px;
    font-weight: 600;
    color: #1f2937;
  }

  .scale-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: #e5e7eb;
    appearance: none;
    cursor: pointer;
  }

  .scale-slider::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #2563eb;
    cursor: pointer;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .scale-slider::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #2563eb;
    cursor: pointer;
    border: none;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: 999px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .status-pill .status-value {
    font-size: 14px;
    font-weight: 700;
    color: #0f172a;
    text-transform: none;
    letter-spacing: -0.01em;
  }

  .map-notices {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 6;
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
    height: 100%;
    display: grid;
    place-items: center;
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
      height: calc(100vh - var(--app-header-height) - var(--app-nav-height));
    }

    .map-overlay.bottom-right {
      flex-direction: column;
      align-items: flex-start;
    }

    .map-overlay.top-right {
      right: 10px;
      left: 10px;
      width: calc(100% - 20px);
    }
  }

  @media (max-width: 640px) {
    .map-overlay.bottom-right {
      bottom: 72px;
      right: 10px;
    }
  }
</style>
