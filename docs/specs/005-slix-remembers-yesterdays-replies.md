# 005 - Slix remembers yesterday's replies

Maro opens Slix several times in a day and yesterday's replies are fetched from X only once, so that the report is instant and a history of his replies builds up over time.

## Acceptance Criteria

1. The first time Maro opens Slix on a given day, it fetches yesterday's replies from X and saves them with everything the report shows.
2. Every later run that same day shows the saved replies and does not contact X at all. The numbers are those from the first run of the day.
3. If Maro made no replies yesterday, that is saved too. Later runs that day say `No replies yesterday.` without contacting X.
4. If X fails on the first run of the day, nothing is saved, and the next run tries X again.
5. Saved days are kept. The history grows day by day and nothing is thrown away.

## Technical Design

### Decisions

- **Storage shape:** one JSON file per day, named after the date the replies belong to (yesterday's local date, e.g. `2026-09-20.json`), holding a plain JSON array of `Reply`. An empty array is how "no replies yesterday" is saved. Splitting by day means history grows by adding files, never rewriting old ones.
- **Writes are atomic:** each save writes to a temp file in the same directory then renames it into place, so a crash mid-write can't leave a half-written day that looks complete.
- **`history` is a plain store, not an orchestrator.** It only knows how to load and save a day; it has no idea what "fetch from X" means. The caller decides whether to load, fetch, or save.
- **Load errors are not swallowed.** A missing file is `Ok(None)` — nothing saved yet, go fetch. A file that exists but fails to parse is `Err`, not treated as "missing". Recovering from a corrupt file is out of scope for now.
- **Directory is injected once.** `History` is constructed with a directory (`History::new(dir)`); it doesn't look up its own path per call. `History::default_dir()` mirrors `Config::path()`'s convention: `dirs::config_dir()/slix/history`.

### Components

1. **`history`** (new module)
   - `pub struct History { dir: PathBuf }`
   - `History::new(dir: PathBuf) -> History`
   - `History::default_dir() -> PathBuf` — `~/.config/slix/history`
   - `History::load(&self, date: NaiveDate) -> Result<Option<Vec<Reply>>, HistoryError>` — `Ok(None)` if the file doesn't exist, `Err` if it exists but can't be read/parsed.
   - `History::save(&self, date: NaiveDate, replies: &[Reply]) -> Result<(), HistoryError>` — writes to a temp file and renames it into place.
   - `HistoryError` mirrors `ConfigError`'s shape (an I/O variant and a parse variant).

2. **`api`** (extended)
   - `Reply` gains `Serialize`/`Deserialize` derives so it round-trips directly as the saved record — no separate wrapper type.

3. **Caller** (today's `write_report`/`run` in `main`)
   - Checks `history.load(date)` for yesterday's date first.
   - `Some(replies)` → render those, no API call.
   - `None` → call the API; on success, `history.save(date, &replies)` then render; on failure, propagate the error and save nothing, so the next run retries.

`report` and `window` are untouched.

### Implementation order (TDD)

1. `history`: `save` then `load` round-trips a list of replies for a date.
2. `history`: `load` for a date with no file returns `Ok(None)`.
3. `history`: `load` for a corrupt file returns `Err`.
4. `history`: saving an empty list round-trips to `Ok(Some(vec![]))`.
5. `history`: `save` is atomic (temp file + rename) — cover by asserting no partial file is left if a write is interrupted, or at minimum that the target file only ever contains complete, valid JSON.
6. `api`: `Reply` (de)serializes via `serde_json` (derive test).
7. Caller: first run of the day with no saved file fetches from the API and saves the result.
8. Caller: a later run the same day loads from `history` and makes no API call.
9. Caller: an API failure on the first run of the day saves nothing; the next run tries the API again.
