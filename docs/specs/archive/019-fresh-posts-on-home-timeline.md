# Fresh posts on my home timeline

## User Story

Marouane opens X and lands on the home timeline. Scrolling through, a small green badge sits in the top right corner of every post that's less than 30 minutes old. Marouane spots one right away and jumps in with an early reply before the post takes off.

## Acceptance Criteria

- On the X home timeline, every post published less than 30 minutes ago shows a green badge in its top right corner.
- The badge has no text.
- Posts 30 minutes old or older show no badge.
- Posts loaded while scrolling get the badge too if they're fresh.
- The badge does not appear anywhere on X except the home timeline.

## Technical Design

The extension is split into a **pure core** (no DOM, unit-tested with Node's built-in runner) and a **thin DOM shell** (verified by hand on x.com).

### Files (`chrome/`)

| File | Role |
|---|---|
| `manifest.json` | Content script `content.js` on `x.com` / `twitter.com`. Adds `web_accessible_resources` for `main.js` and `fresh.js`, matched to `https://x.com/*` and `https://twitter.com/*`. |
| `content.js` | Loader only: `import(chrome.runtime.getURL("main.js"))`. MV3 content scripts can't be ES modules, so everything else is loaded through this dynamic import. Replaces the placeholder badge. |
| `fresh.js` | Pure core, ES module. |
| `main.js` | DOM shell, ES module. |
| `fresh.test.js` | `node --test` tests for `fresh.js`. |
| `package.json` | `{ "type": "module" }` only, so Node treats the `.js` files as ES modules. No dependencies. |

### Core: `fresh.js`

Stateless; knows only the rules.

- `FRESH_MS = 30 * 60 * 1000`
- `isFresh(datetime, now)` returns `true` when `now - Date.parse(datetime) < FRESH_MS`. `now` is passed in (ms since epoch) to keep it deterministic. Exactly 30 min is not fresh. An unparseable or missing `datetime` is not fresh.
- `isHomeTimeline(url)` returns `true` only when the URL's pathname is exactly `/home`. A query string is allowed. `/home/...`, `/{user}`, `/{user}/status/{id}`, `/explore`, `/notifications`, and `/search` are all `false`.

### Shell: `main.js`

Collaborates with `fresh.js` and the page DOM.

- **Startup:** runs one `scan()` for posts already rendered, then creates a single `MutationObserver` on `document.body` (`childList`, `subtree`) that calls `scan()` on every mutation batch. It runs on every x.com page.
- **`scan()`:**
  1. Returns immediately unless `isHomeTimeline(location.href)`. X is an SPA with no navigation event, but navigation always mutates the DOM, so a per-batch check covers landing on another page and then clicking Home.
  2. For each `article[data-testid="tweet"]` that has no `[data-slix-fresh]` child yet, it reads the `datetime` of the article's **first** `<time>`. The first one is used because a quoted post's `<time>` comes later in document order. A repost carries the original post's time. Ads have no `<time>` and are skipped.
  3. If `isFresh(datetime, Date.now())`, it appends the badge.
- **Badge:** an empty `<span data-slix-fresh aria-hidden="true">` with inline styles: a 10px circle in `#00ba7c` with `pointer-events: none`, positioned `absolute` at `top: 6px; right: 6px`. If the article's computed `position` is `static`, the shell sets it to `relative`.
- **No memory:** each article is judged when it is rendered. If X re-creates a post's article after the post has passed 30 minutes (because of virtualized scrolling), it gets no badge. Badges are never removed by us; they leave with the article when X unmounts the timeline.

### Tooling

- `flake.nix`: add `nodejs` to the dev shell.
- `Makefile`: add `test-chrome`, which runs `nix develop --command node --test chrome/`. `make test` runs both `cargo test` and `test-chrome`.

### Tests (`fresh.test.js`)

- `isFresh`: true at 0 min and at 29m59s, false at exactly 30 min and beyond, false for garbage or empty input. Future timestamps (from clock skew) count as fresh.
- `isHomeTimeline`: true for `https://x.com/home`, `https://x.com/home?x=1`, and `https://twitter.com/home`. False for `/home/foo`, `/`, `/{user}`, `/{user}/status/123`, `/explore`, `/notifications`, and `/search?q=a`.
