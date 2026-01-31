export type StoreName =
  | 'sensor_devices'
  | 'sensor_series'
  | 'switch_devices'
  | 'device_states'
  | 'floor_plan'
  | 'meta';

const DB_NAME = 'home_automation_frontend_cache';
const DB_VERSION = 2;

const STORE_DEFS: Array<{ name: StoreName; options?: IDBObjectStoreParameters }> = [
  { name: 'sensor_devices', options: { keyPath: 'device_id' } },
  { name: 'sensor_series', options: { keyPath: 'key' } },
  { name: 'switch_devices', options: { keyPath: 'id' } },
  { name: 'device_states', options: { keyPath: 'device_id' } },
  { name: 'floor_plan', options: { keyPath: 'key' } },
  { name: 'meta' },
];

let dbPromise: Promise<IDBDatabase> | null = null;

function isBrowser(): boolean {
  return typeof window !== 'undefined' && typeof indexedDB !== 'undefined';
}

function openDb(): Promise<IDBDatabase> {
  if (dbPromise) {
    return dbPromise;
  }

  dbPromise = new Promise((resolve, reject) => {
    if (!isBrowser()) {
      reject(new Error('IndexedDB not available'));
      return;
    }

    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onupgradeneeded = () => {
      const db = request.result;
      for (const def of STORE_DEFS) {
        if (!db.objectStoreNames.contains(def.name)) {
          db.createObjectStore(def.name, def.options);
        }
      }
    };

    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('Failed to open IndexedDB'));
  });

  return dbPromise;
}

async function withStore<T>(
  storeName: StoreName,
  mode: IDBTransactionMode,
  fn: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDb();
  return new Promise<T>((resolve, reject) => {
    const transaction = db.transaction(storeName, mode);
    const store = transaction.objectStore(storeName);
    const request = fn(store);

    request.onsuccess = () => resolve(request.result as T);
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
  });
}

export async function getAll<T>(storeName: StoreName): Promise<T[]> {
  if (!isBrowser()) {
    return [];
  }
  return withStore<T[]>(storeName, 'readonly', (store) => store.getAll());
}

export async function get<T>(storeName: StoreName, key: IDBValidKey): Promise<T | undefined> {
  if (!isBrowser()) {
    return undefined;
  }
  return withStore<T | undefined>(storeName, 'readonly', (store) => store.get(key));
}

export async function put<T>(storeName: StoreName, value: T): Promise<void> {
  if (!isBrowser()) {
    return;
  }
  await withStore(storeName, 'readwrite', (store) => store.put(value as any));
}

export async function bulkPut<T>(storeName: StoreName, values: T[]): Promise<void> {
  if (!isBrowser()) {
    return;
  }

  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const transaction = db.transaction(storeName, 'readwrite');
    const store = transaction.objectStore(storeName);

    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error ?? new Error('IndexedDB transaction failed'));

    for (const value of values) {
      store.put(value as any);
    }
  });
}

export async function deleteKey(storeName: StoreName, key: IDBValidKey): Promise<void> {
  if (!isBrowser()) {
    return;
  }
  await withStore(storeName, 'readwrite', (store) => store.delete(key));
}
