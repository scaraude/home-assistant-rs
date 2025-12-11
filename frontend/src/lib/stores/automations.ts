// Automation rules store for managing automation state
import { writable, derived } from 'svelte/store';
import type { AutomationRule } from '../api';

interface AutomationState {
  rules: AutomationRule[];
  loading: boolean;
  error: string | null;
  lastFetch: number | null;
}

function createAutomationStore() {
  const { subscribe, update, set } = writable<AutomationState>({
    rules: [],
    loading: false,
    error: null,
    lastFetch: null,
  });

  return {
    subscribe,

    /**
     * Set all rules (replaces existing)
     */
    setRules(rules: AutomationRule[]) {
      update((state) => ({
        ...state,
        rules,
        lastFetch: Date.now(),
        error: null,
      }));
    },

    /**
     * Set loading state
     */
    setLoading(loading: boolean) {
      update((state) => ({ ...state, loading }));
    },

    /**
     * Set error state
     */
    setError(error: string | null) {
      update((state) => ({ ...state, error }));
    },

    /**
     * Add a new rule
     */
    addRule(rule: AutomationRule) {
      update((state) => ({
        ...state,
        rules: [...state.rules, rule],
      }));
    },

    /**
     * Update an existing rule
     */
    updateRule(id: string, updates: Partial<AutomationRule>) {
      update((state) => ({
        ...state,
        rules: state.rules.map((r) => (r.id === id ? { ...r, ...updates } : r)),
      }));
    },

    /**
     * Remove a rule
     */
    removeRule(id: string) {
      update((state) => ({
        ...state,
        rules: state.rules.filter((r) => r.id !== id),
      }));
    },

    /**
     * Clear all state
     */
    clear() {
      set({
        rules: [],
        loading: false,
        error: null,
        lastFetch: null,
      });
    },
  };
}

export const automationStore = createAutomationStore();

/**
 * Derived store: Group rules by device ID (action device IDs)
 * This allows quick lookup of rules that control a specific switch
 */
export const rulesByDevice = derived(automationStore, ($store) => {
  const byDevice: Record<string, AutomationRule[]> = {};

  for (const rule of $store.rules) {
    // Group by action device IDs (the switches this rule controls)
    for (const action of rule.actions) {
      if (!byDevice[action.device_id]) {
        byDevice[action.device_id] = [];
      }
      // Avoid duplicates if a rule has multiple actions for the same device
      if (!byDevice[action.device_id].some((r) => r.id === rule.id)) {
        byDevice[action.device_id].push(rule);
      }
    }
  }

  return byDevice;
});
