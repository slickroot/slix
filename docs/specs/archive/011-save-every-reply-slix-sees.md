# 011 - Save every reply slix sees

## User Story

Maro skipped slix on Wednesday but replied to a few people that day. On Friday, Maro opens slix, and every reply in the latest 100 posts that wasn't saved yet gets stored under the day it was posted, today included. Maro looks at the Accounts section and sees the accounts from Wednesday's replies counted in the ranking. Happy, Maro knows no reply goes to waste.

## Acceptance Criteria

- Every reply in the fetch that isn't saved yet gets saved under the day it was posted, in your local time
- Today's replies get saved too
- A reply that's already saved is never saved twice
- The Accounts section includes replies from days slix wasn't run

## Technical Design

**Scope constraint:** builds on spec 010's `api.latest_replies(&me.id)` (a single fetch of the latest 100 posts, already filtered to replies). This spec decides what happens to that batch before the report is rendered.

### Decisions

- **Reporting stops touching the API.** `write_report` no longer takes `api`. It only reads from `history` and `today_goal`. If a day isn't in the files, it's rendered as empty (Yesterday: no replies; Accounts: that data isn't there; Today: 0) — never fetched on demand. This is the same "if something is missing, it's not reported" rule for both sections.
- **Saving and reporting are separate steps in `main()`.** A new function, `save_replies`, runs once per invocation, right after the fetch and before `write_report`. It's the only thing that writes to `history` for this spec.
- **`save_replies` groups by local calendar day.** For each fetched reply, the day is `reply.created_at.with_timezone(&Local).date_naive()`. This is what makes a skipped day's replies land under the day they were posted instead of today.
- **No dedup check — always override.** For each local day present in the fetch, `save_replies` just calls `history.save(date, replies_for_that_day)` with whatever the current fetch has for that day, overwriting the file outright. No load-and-merge with what was already saved. This is what makes "already saved twice" a non-issue: the file for a day is always replaced by the fetch's own view of that day, so metrics like impressions naturally refresh too. Days that don't appear in the current fetch are left untouched (their files aren't touched at all).
- **`today.json` gets refreshed too, from `history`, not from a second API concern.** After `save_replies` runs, `main()` derives today's count as `history.load(now.date_naive())?.unwrap_or_default().len()` and saves it to `today_goal`. No separate "is it stale" gate on the write side — 010 already made the fetch happen every run, so the count is recomputed and saved every run.
- **`today_goal` (today.json) itself is untouched.** Its `load`/`save`/staleness logic stays exactly as is; `write_report` still just calls `today_goal.load(now)?.unwrap_or(0)`.
- **`history` (module) is untouched.** `save_replies` is a new caller of its existing `load`/`save`, not a new method on it.
- **`window::Window` isn't used by any of this.** The day math is a plain local-date conversion, not a start/end range.

### Components

1. **`main`** (changed)
   - New function `save_replies(history: &History, replies: Vec<Reply>) -> Result<(), history::HistoryError>`: groups `replies` by local date, and for each date, overwrites via `history.save(date, replies_for_that_day)`.
   - `main()`'s flow becomes: `connect` → `api.latest_replies(&me.id)` → `save_replies(&history, latest)` → derive and save today's count into `today_goal` → `write_report(&history, &me, &today_goal, now, &mut output)`.
   - `write_report` drops its `api: &api::XApiClient` parameter. It keeps `me` (for the "@handle · connected" line). Yesterday becomes `history.load(date)?.unwrap_or_default()`. Today becomes `today_goal.load(now)?.unwrap_or(0)`. Accounts is unchanged (`history.load_all()`).

2. **`history`, `today`, `api`, `report`, `accounts`, `window`** — unchanged by this spec.

### Implementation order (TDD)

1. `main`: `save_replies` with an empty history — saves each fetched reply under its local posted day, including today.
2. `main`: `save_replies` called twice for the same day — the second call's replies for that day replace the file outright (so a same-id reply with refreshed metrics wins, and nothing is ever duplicated).
3. `main`: `save_replies` with a day absent from the current batch — that day's existing file is left untouched.
4. `main`: `write_report` drops `api`; Yesterday and Today read straight from `history`/`today_goal` with no data as empty/zero, no fetch fallback. Update existing `write_report` tests accordingly (they stop mocking `/tweets` entirely for those cases).
5. `main`: acceptance test mirroring the user story — history seeded with nothing, one fetch spanning today and two days back (a skipped day), assert all three days land in `history` under their own dates, Accounts includes the skipped day's account, and `today.json` reflects today's count.
