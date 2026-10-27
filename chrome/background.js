import { decidePing, notificationMessage } from "./ping.js";
import { nextRefreshDelayMs } from "./refresh.js";

const TIMELINE_URLS = ["https://x.com/*", "https://twitter.com/*"];

function schedule() {
  chrome.alarms.create("refresh", { delayInMinutes: nextRefreshDelayMs(Math.random()) / 60000 });
}

chrome.runtime.onInstalled.addListener(schedule);
chrome.runtime.onStartup.addListener(schedule);

chrome.alarms.onAlarm.addListener(async (alarm) => {
  if (alarm.name !== "refresh") return;

  const tabs = await chrome.tabs.query({ url: TIMELINE_URLS });
  for (const tab of tabs) chrome.tabs.sendMessage(tab.id, { type: "refresh" }).catch(() => {});

  schedule();
});

async function ping({ id, url, author, text }) {
  const { pinged = {} } = await chrome.storage.local.get("pinged");
  const decision = decidePing(pinged, id, Date.now());
  await chrome.storage.local.set({ pinged: decision.pinged });
  if (!decision.notify) return;

  await chrome.storage.session.set({ [id]: url });
  await chrome.notifications.create(id, {
    type: "basic",
    iconUrl: "icon-128.png",
    title: author,
    message: notificationMessage(text),
  });
}

// One at a time, so two messages for the same post can't both read `pinged` before either writes it.
let pings = Promise.resolve();

chrome.runtime.onMessage.addListener((message) => {
  if (message?.type !== "takeoff") return;
  pings = pings.then(() => ping(message)).catch(() => {});
});

chrome.notifications.onClicked.addListener(async (id) => {
  const { [id]: url } = await chrome.storage.session.get(id);
  const tab = await chrome.tabs.create({ url });
  await chrome.windows.update(tab.windowId, { focused: true });
  chrome.notifications.clear(id);
});
