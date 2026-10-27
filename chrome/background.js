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
