import { isFresh, isHomeTimeline } from "./fresh.js";

const TWEET = 'article[data-testid="tweet"]';

function badge() {
  const dot = document.createElement("span");
  dot.setAttribute("data-slix-fresh", "");
  dot.setAttribute("aria-hidden", "true");
  Object.assign(dot.style, {
    display: "block",
    width: "10px",
    height: "10px",
    borderRadius: "50%",
    background: "#00ba7c",
    pointerEvents: "none",
    position: "absolute",
    top: "6px",
    right: "6px",
  });
  return dot;
}

function scan() {
  if (!isHomeTimeline(location.href)) return;

  for (const article of document.querySelectorAll(TWEET)) {
    if (article.querySelector("[data-slix-fresh]")) continue;
    const time = article.querySelector("time");
    if (!time || !isFresh(time.getAttribute("datetime"), Date.now())) continue;
    if (getComputedStyle(article).position === "static") article.style.position = "relative";
    article.appendChild(badge());
  }
}

scan();
new MutationObserver(scan).observe(document.body, { childList: true, subtree: true });