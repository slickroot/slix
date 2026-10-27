import { isFresh, isHomeTimeline } from "./fresh.js";
import { isTakingOff, parseViews } from "./takeoff.js";

const TWEET = 'article[data-testid="tweet"]';
const ACTION_BAR = '[role="group"][aria-label]';

function dot(attr, color, order) {
  const el = document.createElement("span");
  el.setAttribute(attr, "");
  Object.assign(el.style, {
    display: "block",
    width: "10px",
    height: "10px",
    borderRadius: "50%",
    background: color,
    order: String(order),
  });
  return el;
}

function dotContainer(article) {
  if (getComputedStyle(article).position === "static") article.style.position = "relative";
  const container = document.createElement("span");
  container.setAttribute("data-slix-dots", "");
  container.setAttribute("aria-hidden", "true");
  Object.assign(container.style, {
    position: "absolute",
    top: "6px",
    right: "6px",
    display: "flex",
    gap: "4px",
    pointerEvents: "none",
  });
  article.appendChild(container);
  return container;
}

function scan() {
  if (!isHomeTimeline(location.href)) return;

  const now = Date.now();

  for (const article of document.querySelectorAll(TWEET)) {
    const time = article.querySelector("time");
    if (!time) continue;

    const datetime = time.getAttribute("datetime");
    const label = article.querySelector(ACTION_BAR)?.getAttribute("aria-label");

    if (!article.querySelector("[data-slix-fresh]") && isFresh(datetime, now)) {
      const dots = article.querySelector("[data-slix-dots]") || dotContainer(article);
      dots.appendChild(dot("data-slix-fresh", "#00ba7c", 1));
    }

    if (!article.querySelector("[data-slix-takeoff]") && isTakingOff(datetime, parseViews(label), now)) {
      const dots = article.querySelector("[data-slix-dots]") || dotContainer(article);
      dots.appendChild(dot("data-slix-takeoff", "#ff7a00", 0));
    }
  }
}

scan();
new MutationObserver(scan).observe(document.body, { childList: true, subtree: true });