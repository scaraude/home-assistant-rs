<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";
  import type { Node, NodeProps } from "@xyflow/svelte";
  import { setDeviceOption } from "../api";
  import EditableDeviceName from "../EditableDeviceName.svelte";
  import StatusBadge from "../StatusBadge.svelte";

  type DeviceData = Record<string, unknown> & {
    device: {
      id: string;
      name: string;
      mqtt_topic: string;
      capability: {
        type: "sensor" | "commander" | "coordinator";
        sensor_type?: "temp_humidity" | "presence";
        commander_type?: "switch";
      };
      power_source: "battery" | "plugged";
      is_bridge: boolean;
      parent_device_id: string | null;
    };
    deviceState?: {
      link_quality: number | null;
      battery_level: number | null;
      turbo_mode?: boolean | null;
    };
    onTurboModeChange?: (deviceId: string, turboMode: boolean) => void;
  };

  type DeviceNode = Node<DeviceData>;

  let { data }: NodeProps<DeviceNode> = $props();

  const isCoordinator = $derived(!data.device.parent_device_id);
  const isBridge = $derived(data.device.is_bridge);
  const supportsTurbo = $derived(isBridge);
  const linkQuality = $derived(data.deviceState?.link_quality);
  const batteryLevel = $derived(data.deviceState?.battery_level);
  const turboFromState = $derived(data.deviceState?.turbo_mode ?? false);
  let turboEnabled = $state<boolean>(data.deviceState?.turbo_mode ?? false);
  let turboError = $state<string | null>(null);
  let turboUpdating = $state(false);

  $effect(() => {
    if (!turboUpdating) {
      turboEnabled = turboFromState;
    }
  });

  function getIcon(device: typeof data.device): string {
    if (device.capability.type === "sensor") {
      if (device.capability.sensor_type === "temp_humidity") {
        return "🌡️";
      } else if (device.capability.sensor_type === "presence") {
        return "💡";
      }
    } else if (device.capability.type === "commander") {
      if (device.capability.commander_type === "switch") {
        return "⚙️";
      }
    } else if (device.capability.type === "coordinator") {
      return "📡";
    }
    return "📱";
  }

  function getLinkQualityColor(lqi: number | null | undefined): string {
    if (lqi == null) return "#6b7280"; // gray
    if (lqi > 100) return "#22c55e"; // green
    if (lqi >= 50) return "#eab308"; // yellow
    return "#ef4444"; // red
  }

  const borderColor = $derived(
    isCoordinator ? "#f59e0b" : isBridge ? "#3b82f6" : "#6b7280",
  );

  async function toggleTurboMode() {
    if (turboUpdating || !supportsTurbo) return;

    turboUpdating = true;
    turboError = null;

    const previous = turboEnabled;
    const next = !turboEnabled;
    turboEnabled = next;

    try {
      await setDeviceOption(data.device.id, "turbo_mode", next);
      data.onTurboModeChange?.(data.device.id, next);
    } catch (error) {
      turboEnabled = previous;
      turboError =
        error instanceof Error ? error.message : "Turbo update failed";
    } finally {
      turboUpdating = false;
    }
  }
</script>

<div class="device-node" style="border-color: {borderColor}">
  {#if !isCoordinator}
    <Handle type="target" position={Position.Top} />
  {/if}

  <div class="node-content">
    <div class="icon">{getIcon(data.device)}</div>

    <div class="info">
      <EditableDeviceName deviceId={data.device.id} name={data.device.name} />

      <div class="badges">
        {#if isCoordinator}
          <span class="badge coordinator">Coordinator</span>
        {:else if isBridge}
          <span class="badge bridge">Router</span>
        {/if}

        {#if data.device.power_source === "battery" && batteryLevel != null}
          <StatusBadge value={batteryLevel} type="battery" />
        {/if}

        {#if linkQuality != null}
          <span
            class="badge link-quality"
            style="background-color: {getLinkQualityColor(linkQuality)};"
          >
            LQI: {linkQuality}
          </span>
        {/if}
      </div>

      {#if supportsTurbo}
        <div class="turbo-row">
          <span class="turbo-label">Turbo</span>
          <label class="turbo-toggle">
            <input
              type="checkbox"
              checked={turboEnabled}
              disabled={turboUpdating}
              onchange={toggleTurboMode}
            />
            <span class="toggle-track"></span>
          </label>
        </div>
        {#if turboError}
          <div class="turbo-error">{turboError}</div>
        {/if}
      {/if}
    </div>
  </div>

  {#if isBridge || isCoordinator}
    <Handle type="source" position={Position.Bottom} />
  {/if}
</div>

<style>
  .device-node {
    background: white;
    border: 2px solid;
    border-radius: 8px;
    padding: 12px;
    min-width: 200px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    transition: box-shadow 0.2s;
  }

  .device-node:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .node-content {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .icon {
    font-size: 32px;
    line-height: 1;
  }

  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
    color: white;
  }

  .badge.coordinator {
    background-color: #f59e0b;
  }

  .badge.bridge {
    background-color: #3b82f6;
  }

  .badge.link-quality {
    color: white;
  }

  .turbo-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .turbo-label {
    font-size: 12px;
    font-weight: 600;
    color: #374151;
  }

  .turbo-toggle {
    position: relative;
    display: inline-flex;
    align-items: center;
    cursor: pointer;
  }

  .turbo-toggle input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }

  .toggle-track {
    width: 34px;
    height: 18px;
    background: #e5e7eb;
    border-radius: 999px;
    position: relative;
    transition: background 0.2s ease;
  }

  .toggle-track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: white;
    border-radius: 999px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: transform 0.2s ease;
  }

  .turbo-toggle input:checked + .toggle-track {
    background: #22c55e;
  }

  .turbo-toggle input:checked + .toggle-track::after {
    transform: translateX(16px);
  }

  .turbo-toggle input:disabled + .toggle-track {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .turbo-error {
    font-size: 11px;
    color: #b91c1c;
  }
</style>
