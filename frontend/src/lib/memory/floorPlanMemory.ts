import { writable } from 'svelte/store';
import { fetchFloorPlan, uploadFloorPlan } from '../api/floor-map';
import { deleteKey, get, put } from './indexedDb';

interface FloorPlanState {
  svgContent: string | null;
  uploadedAt: number | null;
  loaded: boolean;
  hydrated: boolean;
}

type PersistedFloorPlan = {
  key: 'floor_plan';
  svg_content: string;
  uploaded_at: number;
};

const STORAGE_KEY: PersistedFloorPlan['key'] = 'floor_plan';

const initialState: FloorPlanState = {
  svgContent: null,
  uploadedAt: null,
  loaded: false,
  hydrated: false,
};

const { subscribe, update } = writable<FloorPlanState>(initialState);

async function hydrate() {
  try {
    const cached = await get<PersistedFloorPlan>('floor_plan', STORAGE_KEY);
    if (cached) {
      update((state) => ({
        ...state,
        svgContent: cached.svg_content,
        uploadedAt: cached.uploaded_at,
        loaded: true,
        hydrated: true,
      }));
      return;
    }
  } catch (error) {
    console.warn('Failed to hydrate floor plan:', error);
  }

  update((state) => ({ ...state, hydrated: true }));
}

async function ensureFloorPlan(force = false): Promise<void> {
  let loaded = false;
  subscribe((state) => {
    loaded = state.loaded;
  })();

  if (loaded && !force) {
    return;
  }

  const plan = await fetchFloorPlan();
  update((state) => ({
    ...state,
    svgContent: plan?.svg_content ?? null,
    uploadedAt: plan?.uploaded_at ?? null,
    loaded: true,
  }));

  try {
    if (plan) {
      const payload: PersistedFloorPlan = {
        key: STORAGE_KEY,
        svg_content: plan.svg_content,
        uploaded_at: plan.uploaded_at,
      };
      await put('floor_plan', payload);
    } else {
      await deleteKey('floor_plan', STORAGE_KEY);
    }
  } catch (error) {
    console.warn('Failed to persist floor plan:', error);
  }
}

async function setFloorPlan(svgContent: string, uploadedAt: number): Promise<void> {
  update((state) => ({
    ...state,
    svgContent,
    uploadedAt,
    loaded: true,
  }));

  try {
    const payload: PersistedFloorPlan = {
      key: STORAGE_KEY,
      svg_content: svgContent,
      uploaded_at: uploadedAt,
    };
    await put('floor_plan', payload);
  } catch (error) {
    console.warn('Failed to persist floor plan:', error);
  }
}

async function uploadNewFloorPlan(svgContent: string): Promise<void> {
  await uploadFloorPlan(svgContent);
  const uploadedAt = Math.floor(Date.now() / 1000);
  await setFloorPlan(svgContent, uploadedAt);
}

export const floorPlanMemory = {
  subscribe,
  hydrate,
  ensureFloorPlan,
  setFloorPlan,
  uploadNewFloorPlan,
};

void hydrate();
