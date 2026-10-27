export const PING_MEMORY_MS = 24 * 60 * 60 * 1000;
export const NOTIFICATION_TEXT_MAX = 120;

export function statusId(href) {
  const match = /\/status\/(\d+)/.exec(href ?? "");
  return match ? match[1] : null;
}

export function decidePing(pinged, id, now) {
  const kept = Object.fromEntries(
    Object.entries(pinged).filter(([, pingedAt]) => now - pingedAt < PING_MEMORY_MS),
  );
  if (id in kept) return { notify: false, pinged: kept };
  return { notify: true, pinged: { ...kept, [id]: now } };
}

export function notificationMessage(text) {
  const trimmed = (text ?? "").trim();
  if (trimmed.length <= NOTIFICATION_TEXT_MAX) return trimmed;
  return trimmed.slice(0, NOTIFICATION_TEXT_MAX - 1).trimEnd() + "…";
}
