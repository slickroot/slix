export const FRESH_MS = 30 * 60 * 1000;

export function isFresh(datetime, now) {
  const published = Date.parse(datetime);
  return now - published < FRESH_MS;
}

export function isHomeTimeline(url) {
  return new URL(url).pathname === "/home";
}