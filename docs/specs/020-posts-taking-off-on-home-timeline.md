# Posts taking off on my home timeline

## User Story

Marouane opens X and lands on the home timeline. Scrolling through, a small orange dot sits in the top right corner of a post from 40 minutes ago that's already pulling over 1000 views an hour. Marouane jumps in with a reply while the post is taking off, and people reading the thread click through to Marouane's profile and follow.

## Acceptance Criteria

- On the X home timeline, every post that is less than 2 hours old and averaging 1000 or more views per hour since it was posted shows an orange dot in its top right corner.
- Views per hour = the post's view count ÷ hours since it was posted.
- The dot has no text.
- Posts 2 hours old or older show no orange dot, however fast they're getting views.
- When a post also has the green fresh dot, the orange dot sits just to the left of the green one.
- Each post is judged once, when it loads onto the timeline. The dot doesn't appear or disappear while the post is on screen.
- Posts loaded while scrolling get the orange dot too if they qualify.
- The orange dot does not appear anywhere on X except the home timeline.

## Technical Design

Builds on spec 019: a new rule module in the **pure core** and a few changes to the **DOM shell** in `main.js`.

### Files (`chrome/`)

| File | Change |
|---|---|
| `takeoff.js` | New pure core ES module for the take-off rules. |
| `takeoff.test.js` | New `node --test` tests for `takeoff.js`. |
| `main.js` | Reads view counts, adds the orange dot, and moves both dots into a shared container. |
| `manifest.json` | Adds `takeoff.js` to `web_accessible_resources`. |

`fresh.js` and its tests stay as they are. `isHomeTimeline` stays in `fresh.js`.

### Core: `takeoff.js`

Stateless; knows only the rules.

- `TAKEOFF_MAX_AGE_MS = 2 * 60 * 60 * 1000`
- `TAKEOFF_VIEWS_PER_HOUR = 1000`
- `parseViews(label)` reads the action bar's `aria-label`, e.g. `"48 replies, 5 reposts, 271 likes, 17 bookmarks, 30107 views"`, and returns the view count as an integer (`30107`). It matches `/(\d[\d,]*)\s+views?\b/i` and strips commas. It returns `null` when there is no match: views are hidden, the post is an ad, the UI isn't in English, or the post has 0 views (X leaves the part out). It also returns `null` for an empty or missing label.
- `isTakingOff(datetime, views, now)` returns `true` when:
  - `views` is a number (not `null`), and
  - `datetime` parses, and
  - `now - published < TAKEOFF_MAX_AGE_MS` (exactly 2 h does not qualify), and
  - `views / hours >= TAKEOFF_VIEWS_PER_HOUR` (exactly 1000 qualifies), where `hours = max((now - published) / 3600000, 1 / 60)`.

  The 1-minute floor keeps posts only a few seconds old from qualifying on a handful of views and avoids dividing by zero. Future timestamps (from clock skew) fall to the floor too. `now` is passed in (ms since epoch) to keep the function deterministic.

### Shell: `main.js`

Collaborates with `fresh.js`, `takeoff.js`, and the page DOM. Startup, the `MutationObserver`, and the `isHomeTimeline` guard are unchanged from 019.

- **`scan()`:** for each `article[data-testid="tweet"]`, it reads the `datetime` of the article's first `<time>` (as in 019) and the `aria-label` of the article's first `[role="group"][aria-label]`, which is the action bar. Quoted posts have no action bar of their own. With `now = Date.now()`:
  - If the article has no `[data-slix-fresh]` and `isFresh(datetime, now)`, it adds the green dot.
  - If the article has no `[data-slix-takeoff]` and `isTakingOff(datetime, parseViews(label), now)`, it adds the orange dot.
- **No memory:** nothing marks an article as judged. Every scan re-judges every article that doesn't have the dot yet, as in 019. The action bar may render after the `<time>`, so an article's orange dot can show up on a later scan, and a post on screen can gain the dot if X live-updates its view count. Dots are never removed; they leave with the article when X unmounts it.
- **Dot container:** the first dot an article gets creates `<span data-slix-dots aria-hidden="true">`, appended to the article. It is styled `position: absolute; top: 6px; right: 6px; display: flex; gap: 4px; pointer-events: none`. If the article's computed `position` is `static`, the shell sets it to `relative` (moved here from 019's badge code). Later dots go into the existing container.
- **Dots:** `dot(attr, color, order)` replaces `badge()` and makes an empty `<span>` with the given data attribute: a 10px circle (`display: block; border-radius: 50%`) with CSS `order` set. No text.
  - Green: `data-slix-fresh`, `#00ba7c`, `order: 1`.
  - Orange: `data-slix-takeoff`, `#ff7a00`, `order: 0`.

  Because the container uses flex `order`, the orange dot always sits left of the green one, whichever was inserted first. A single dot sits in the corner.

### Tests (`takeoff.test.js`)

- `parseViews`: `30107` for the full group label above; `1` for `"1 view"`; `12345` for `"12,345 views"`; `null` for a label with no views part, for `""`, and for `undefined`.
- `isTakingOff` (with `NOW` fixed), view counts derived from `TAKEOFF_VIEWS_PER_HOUR` and the elapsed hours:
  - True at 40 min with 667 views (1000.5/hour), false at 40 min with 666 views (999/hour).
  - True just under 2 h with plenty of views; false at exactly 2 h and beyond, however many views.
  - At 10 s old: false with 16 views, true with 17 (1-minute floor).
  - A future datetime uses the floor: true with 17 views, false with 16.
  - False for `null` views, an unparseable datetime, an empty datetime, or a missing datetime.

The shell (dot container, ordering, live re-judging) is verified by hand on x.com/home.

Amendment, 6 October 2026: reviewing PR #19 lowered the threshold from 100 views per minute to 1000 views per hour (~16.7 per minute), so posts that are picking up real but not viral traffic get the dot too.
