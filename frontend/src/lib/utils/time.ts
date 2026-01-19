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
