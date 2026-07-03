import { writable } from 'svelte/store';
import { RECOMMENDED_COLORS, type GraphState } from '../stores/graphConfig';
import { seriesKey, type ExplorerMetric } from './metrics';

export type ExplorerTimeRange = GraphState['timeRange'];

export interface ExplorerSeries {
  deviceId: string;
  metric: ExplorerMetric;
  color: string;
}

export interface ExplorerState {
  /** selected series, in insertion order (drives dataset + legend order) */
  series: ExplorerSeries[];
  timeRange: ExplorerTimeRange;
  /**
   * Reserved for a future "normalize to 0–100%" toggle when many units are on
   * screen. Not surfaced in the UI yet — see docs / issue backlog.
   */
  normalize: boolean;
}

const STORAGE_KEY = 'homeAutomation:explorerConfig';

const initialState: ExplorerState = {
  series: [],
  timeRange: '24h',
  normalize: false,
};

function loadPersisted(): Partial<ExplorerState> {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return {};
    const parsed = JSON.parse(stored);
    return {
      series: Array.isArray(parsed.series) ? parsed.series : [],
      timeRange: parsed.timeRange ?? '24h',
      normalize: !!parsed.normalize,
    };
  } catch (e) {
    console.warn('Failed to load explorer config:', e);
    return {};
  }
}

function createExplorerConfig() {
  const { subscribe, update } = writable<ExplorerState>({
    ...initialState,
    ...loadPersisted(),
  });

  let saveTimeout: ReturnType<typeof setTimeout>;
  function persist(state: ExplorerState) {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
      } catch (e) {
        console.warn('Failed to persist explorer config:', e);
      }
    }, 400);
  }

  function nextColor(state: ExplorerState, preferred?: string | null): string {
    if (preferred && preferred.trim()) return preferred;
    const used = new Set(state.series.map((s) => s.color));
    const free = RECOMMENDED_COLORS.find((c) => !used.has(c));
    return free ?? RECOMMENDED_COLORS[state.series.length % RECOMMENDED_COLORS.length];
  }

  return {
    subscribe,

    /** Add or remove a (device, metric) series. */
    toggleSeries(deviceId: string, metric: ExplorerMetric, preferredColor?: string | null) {
      update((state) => {
        const key = seriesKey(deviceId, metric);
        const exists = state.series.some((s) => seriesKey(s.deviceId, s.metric) === key);
        const series = exists
          ? state.series.filter((s) => seriesKey(s.deviceId, s.metric) !== key)
          : [...state.series, { deviceId, metric, color: nextColor(state, preferredColor) }];
        const next = { ...state, series };
        persist(next);
        return next;
      });
    },

    removeSeries(deviceId: string, metric: ExplorerMetric) {
      update((state) => {
        const key = seriesKey(deviceId, metric);
        const next = {
          ...state,
          series: state.series.filter((s) => seriesKey(s.deviceId, s.metric) !== key),
        };
        persist(next);
        return next;
      });
    },

    setColor(deviceId: string, metric: ExplorerMetric, color: string) {
      update((state) => {
        const key = seriesKey(deviceId, metric);
        const next = {
          ...state,
          series: state.series.map((s) =>
            seriesKey(s.deviceId, s.metric) === key ? { ...s, color } : s,
          ),
        };
        persist(next);
        return next;
      });
    },

    clear() {
      update((state) => {
        const next = { ...state, series: [] };
        persist(next);
        return next;
      });
    },

    setTimeRange(timeRange: ExplorerTimeRange) {
      update((state) => {
        const next = { ...state, timeRange };
        persist(next);
        return next;
      });
    },

    setNormalize(normalize: boolean) {
      update((state) => {
        const next = { ...state, normalize };
        persist(next);
        return next;
      });
    },
  };
}

export const explorerConfig = createExplorerConfig();
