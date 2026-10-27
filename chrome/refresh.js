import { isHomeTimeline } from "./fresh.js";

export const REFRESH_MIN_MS = 5 * 60 * 1000;
export const REFRESH_MAX_MS = 10 * 60 * 1000;

export function nextRefreshDelayMs(random) {
  return REFRESH_MIN_MS + random * (REFRESH_MAX_MS - REFRESH_MIN_MS);
}

export function shouldReload(url, hasFocus) {
  return isHomeTimeline(url) && !hasFocus;
}
