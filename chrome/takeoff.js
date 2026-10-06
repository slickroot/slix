export const TAKEOFF_MAX_AGE_MS = 2 * 60 * 60 * 1000;
export const TAKEOFF_VIEWS_PER_MIN = 100;

const VIEWS_PATTERN = /(\d[\d,]*)\s+views?\b/i;

export function parseViews(label) {
  if (!label) {
    return null;
  }

  const match = String(label).match(VIEWS_PATTERN);
  if (!match) {
    return null;
  }

  return Number(match[1].replaceAll(",", ""));
}

export function isTakingOff(datetime, views, now) {
  if (typeof views !== "number") {
    return false;
  }

  const published = Date.parse(datetime);
  if (Number.isNaN(published)) {
    return false;
  }

  const age = now - published;
  if (age >= TAKEOFF_MAX_AGE_MS) {
    return false;
  }

  const minutes = Math.max(age / 60000, 1);
  return views / minutes >= TAKEOFF_VIEWS_PER_MIN;
}
