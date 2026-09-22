# 006 - Today's reply goal

## User Story

As Slix, when I run the report, I want to see today's reply count against my daily goal of 5, so I know how much more I need to do today.

## Acceptance Criteria

- Below the "Yesterday" section, a new line reads: `Today: {count} of 5 replies ({goal - count} to go)`
- Once the count reaches or passes 5, it instead reads: `Today: {count} of 5 replies (goal met!)`
- Today's reply count is fetched from the API and cached for 1 hour before being refetched
- The daily goal is fixed at 5 replies

## Technical Design

### Decisions

- **Reuse `XApiClient::replies` for today's count.** No dedicated count endpoint — fetch today's replies the same way yesterday's are fetched, and take `.len()`. `Window::today(now)` is added alongside `Window::yesterday(now)`: local midnight today through `now`.
- **A new, separate store owns the 1-hour cache — not `History`.** `History` stays a plain per-day, write-once data source. Today's count is a different shape entirely: it's the *current* day only, gets overwritten repeatedly through the day, and expires after an hour. Mixing that into `History` would break its "write-once, never rewritten" nature.
- **The new store is minimal: `count` + `fetched_at`, nothing else.** It does not persist the day's `Reply` list, only the derived number and when it was fetched. `fetched_at` is stored as `DateTime<Utc>`, mirroring `Reply::created_at`.
- **Single fixed file, not a directory of per-day files.** There's only ever one "today," so `TodayGoal` is constructed with a file path, not a directory.
- **Staleness is decided inside the store, not the caller.** `TodayGoal::load(&self, now: DateTime<Local>) -> Option<u64>` returns `Some(count)` only when the cache is fresh; `None` covers both "no file yet" and "stale," same as `History::load`'s `None` means "go fetch." A cache is stale if more than 1 hour has passed *or* if `fetched_at`'s local date differs from `now`'s local date (day rollover always invalidates, regardless of how recent `fetched_at` is).
- **Writes are atomic**, same temp-file-then-rename approach as `History::save`.
- **Storage paths move into `Config`.** `Config` gains `data_dir: PathBuf` (serde default in code, computed the same way `Config::path()`'s parent already is: `dirs::config_dir()/slix`), so both `History`'s directory and `TodayGoal`'s file path are derived from one user-editable place instead of each module computing its own default independently. `main.rs` builds `data_dir.join("history")` and `data_dir.join("today.json")` and injects them into `History::new` / `TodayGoal::new`, the same "computed once, injected" pattern `History` already uses.
- **Rendering is a second, independent function**, not folded into the existing one. `report::render_today(count: u64) -> String` sits next to `report::render(replies, tz)`; `write_report` calls both separately and prints today's line after yesterday's block. `const DAILY_GOAL: u64 = 5;` lives in `report.rs` next to it, since that's the only place "5" and the to-go/goal-met wording are used.
- **A fetch failure for today behaves exactly like a fetch failure for yesterday**: propagate the error, abort the whole report, save nothing, retry next run.

### Components

1. **`window`** (extended)
   - `Window::today<Tz: TimeZone>(now: DateTime<Tz>) -> Window` — local midnight today through `now`.

2. **`config`** (extended)
   - `Config` gains `data_dir: PathBuf`, `#[serde(default = "...")]` pointing at `dirs::config_dir()/slix`. Old `config.json` files without the key keep loading.

3. **`today`** (new module)
   - `pub struct TodayGoal { path: PathBuf }`
   - `TodayGoal::new(path: PathBuf) -> TodayGoal`
   - `TodayGoal::load(&self, now: DateTime<Local>) -> Result<Option<u64>, TodayGoalError>` — `Ok(None)` if missing or stale (>1 hour old, or `fetched_at`'s local date != `now`'s local date); `Err` if the file exists but fails to parse.
   - `TodayGoal::save(&self, count: u64, now: DateTime<Local>) -> Result<(), TodayGoalError>` — writes `{ count, fetched_at }` via temp file + rename.
   - `TodayGoalError` mirrors `HistoryError`'s shape (Io / Json).

4. **`report`** (extended)
   - `const DAILY_GOAL: u64 = 5;`
   - `pub fn render_today(count: u64) -> String` — `"Today: {count} of {DAILY_GOAL} replies ({DAILY_GOAL - count} to go)"`, or `"...(goal met!)"` once `count >= DAILY_GOAL`.

5. **Caller (`main::write_report`)**
   - After rendering yesterday's block, checks `today_goal.load(now)`.
   - `Some(count)` → use it, no API call.
   - `None` → call `api.replies(&me.id, &Window::today(now))`, take `.len()` as the count, `today_goal.save(count, now)`, then use it. On API failure, propagate the error and save nothing (same as yesterday's failure path).
   - Prints `report::render_today(count)` after the yesterday block.

### Implementation order (TDD)

1. `window`: `Window::today(now)` spans local midnight to `now`.
2. `report`: `render_today` — below goal, at goal, above goal, zero replies.
3. `today`: `save` then `load` round-trips `{ count, fetched_at }`.
4. `today`: `load` with no file returns `Ok(None)`.
5. `today`: `load` for a corrupt file returns `Err`.
6. `today`: `load` returns `None` when `fetched_at` is more than 1 hour before `now`.
7. `today`: `load` returns `None` when `fetched_at` is on a different local date than `now`, even if less than 1 hour old.
8. `today`: `load` returns `Some(count)` when fresh (same date, within the hour).
9. `today`: `save` is atomic, same coverage style as `History::save`.
10. `config`: `data_dir` round-trips; loading a config file without `data_dir` falls back to the computed default.
11. Caller: no cached file fetches from the API, saves the count, and prints the "Today" line.
12. Caller: a cached, fresh count is reused with no API call.
13. Caller: an API failure fetching today's count saves nothing and propagates the error.
