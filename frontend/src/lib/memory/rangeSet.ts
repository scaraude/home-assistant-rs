export type Range = { start: number; end: number };

function normalizeRange(start: number, end: number): Range {
  if (start > end) {
    return { start: end, end: start };
  }
  return { start, end };
}

export function mergeRanges(ranges: Range[]): Range[] {
  if (ranges.length === 0) return [];
  const sorted = [...ranges].sort((a, b) => a.start - b.start);
  const merged: Range[] = [sorted[0]];
  for (let i = 1; i < sorted.length; i += 1) {
    const current = sorted[i];
    const last = merged[merged.length - 1];
    if (current.start <= last.end + 1) {
      last.end = Math.max(last.end, current.end);
    } else {
      merged.push({ ...current });
    }
  }
  return merged;
}

export function addRange(ranges: Range[], start: number, end: number): Range[] {
  const next = normalizeRange(start, end);
  return mergeRanges([...ranges, next]);
}

export function coversRange(ranges: Range[], start: number, end: number): boolean {
  if (ranges.length === 0) return false;
  const target = normalizeRange(start, end);
  for (const range of ranges) {
    if (range.start <= target.start && range.end >= target.end) {
      return true;
    }
  }
  return false;
}

export function getMissingRanges(ranges: Range[], start: number, end: number): Range[] {
  const target = normalizeRange(start, end);
  if (ranges.length === 0) {
    return [target];
  }

  const merged = mergeRanges(ranges);
  const missing: Range[] = [];
  let cursor = target.start;

  for (const range of merged) {
    if (range.end < cursor) {
      continue;
    }
    if (range.start > target.end) {
      break;
    }

    if (range.start > cursor) {
      missing.push({ start: cursor, end: Math.min(range.start - 1, target.end) });
    }
    cursor = Math.max(cursor, range.end + 1);
    if (cursor > target.end) {
      break;
    }
  }

  if (cursor <= target.end) {
    missing.push({ start: cursor, end: target.end });
  }

  return missing;
}
