/**
 * API functions for floor map features (floor plan SVG and device positions)
 */

export interface FloorPlan {
  svg_content: string;
  uploaded_at: number;
}

export interface DevicePosition {
  device_id: string;
  x: number;
  y: number;
  updated_at: number;
}

/**
 * Fetch the floor plan SVG from the backend
 * @returns The floor plan data or null if not found
 */
export async function fetchFloorPlan(): Promise<FloorPlan | null> {
  const response = await fetch("/api/floor-plan");

  if (response.status === 404) {
    return null;
  }

  if (!response.ok) {
    throw new Error(`Failed to fetch floor plan: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Upload a new floor plan SVG
 * @param svgContent - The SVG content as a string
 */
export async function uploadFloorPlan(svgContent: string): Promise<void> {
  const response = await fetch("/api/floor-plan", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ svg_content: svgContent }),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: "Unknown error" }));
    throw new Error(errorData.error || `Failed to upload floor plan: ${response.statusText}`);
  }
}

/**
 * Delete the floor plan
 */
export async function deleteFloorPlan(): Promise<void> {
  const response = await fetch("/api/floor-plan", {
    method: "DELETE",
  });

  if (!response.ok && response.status !== 404) {
    throw new Error(`Failed to delete floor plan: ${response.statusText}`);
  }
}

/**
 * Fetch all device positions
 * @returns Array of device positions
 */
export async function fetchDevicePositions(): Promise<DevicePosition[]> {
  const response = await fetch("/api/devices/positions");

  if (!response.ok) {
    throw new Error(`Failed to fetch device positions: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Update a device's position on the floor map
 * @param deviceId - The device ID
 * @param x - X coordinate
 * @param y - Y coordinate
 */
export async function updateDevicePosition(
  deviceId: string,
  x: number,
  y: number
): Promise<void> {
  const response = await fetch(`/api/devices/${deviceId}/position`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ x, y }),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: "Unknown error" }));
    throw new Error(errorData.error || `Failed to update device position: ${response.statusText}`);
  }
}
