export interface StorageCategory {
  id: string;
  label: string;
  bytes: number;
}

export interface RamStorageBreakdown {
  total_bytes: number;
  free_bytes: number;
  used_bytes: number;
}

export interface StorageBreakdown {
  mount: string;
  timestamp: string;
  total_bytes: number;
  free_bytes: number;
  used_bytes: number;
  categories: StorageCategory[];
  ram_storage: RamStorageBreakdown | null;
}

export async function fetchStorageBreakdown(): Promise<StorageBreakdown> {
  const response = await fetch('/api/system/storage');
  if (!response.ok) {
    throw new Error(`Failed to fetch storage breakdown: ${response.statusText}`);
  }
  return response.json();
}
