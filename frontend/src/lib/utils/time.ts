export function unixSecondsToDate(seconds: number): Date {
  return new Date(seconds * 1000);
}

export function dateToUnixSeconds(date: Date): number {
  return Math.floor(date.getTime() / 1000);
}

export function parseUnixSeconds(seconds: number | null | undefined): Date | null {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) {
    return null;
  }
  return unixSecondsToDate(seconds);
}

// ============================================================================
// Time-of-day formatting (issue #16)
//
// Time was displayed inconsistently across the app: Chart.js axes used 24h
// (date-fns "HH:mm") while tooltips/tables used `toLocaleString()` (12h under
// en-US). Everything now goes through these helpers so the format is uniform
// and follows a single user preference (defaulting to the browser locale).
// ============================================================================

import { writable } from 'svelte/store';

export type HourCycle = 'auto' | '12h' | '24h';

const HOUR_CYCLE_KEY = 'homeAutomation:hourCycle';

function loadHourCycle(): HourCycle {
  try {
    const v = localStorage.getItem(HOUR_CYCLE_KEY);
    if (v === '12h' || v === '24h' || v === 'auto') return v;
  } catch {
    /* localStorage unavailable (SSR) */
  }
  return 'auto';
}

/** User-selected clock format. Persisted; drives every time display. */
export const hourCycle = writable<HourCycle>(loadHourCycle());

let currentHourCycle: HourCycle = loadHourCycle();
hourCycle.subscribe((v) => {
  currentHourCycle = v;
  try {
    localStorage.setItem(HOUR_CYCLE_KEY, v);
  } catch {
    /* ignore */
  }
});

/** Whether the current locale uses a 12-hour clock. */
function localeUses12h(): boolean {
  try {
    return (
      new Intl.DateTimeFormat(undefined, { hour: 'numeric' }).resolvedOptions().hour12 ?? false
    );
  } catch {
    return false;
  }
}

/** Effective 12h flag: `true`/`false` for explicit prefs, locale default for 'auto'. */
export function useHour12(): boolean {
  if (currentHourCycle === '12h') return true;
  if (currentHourCycle === '24h') return false;
  return localeUses12h();
}

/** Time only, e.g. "18:00" or "6:00 PM". */
export function formatTime(date: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    hour12: useHour12(),
  }).format(date);
}

/** Date + time, for tooltips and tables, e.g. "Jul 2, 2026, 18:00". */
export function formatDateTime(date: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: useHour12(),
  }).format(date);
}

/** Date only, e.g. "Jul 2, 2026". */
export function formatDate(date: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(date);
}

/** date-fns token for the time-of-day part, respecting the hour-cycle pref. */
export function timeToken(): string {
  return useHour12() ? 'h:mm a' : 'HH:mm';
}

/** Chart.js (date-fns) time-axis display formats for a given range. */
export function chartTimeFormats(range: string): Record<string, string> {
  const t = timeToken();
  switch (range) {
    case '24h':
      return { minute: t, hour: t, day: t };
    case '1w':
      return { minute: t, hour: t, day: 'MMM dd' };
    case '1m':
      return { minute: t, hour: t, day: 'MMM dd', week: 'MMM dd' };
    case '1y':
      return { minute: t, hour: t, day: 'MMM dd', week: 'MMM yyyy', month: 'MMM yyyy' };
    default:
      return { hour: t, day: 'MMM dd' };
  }
}
