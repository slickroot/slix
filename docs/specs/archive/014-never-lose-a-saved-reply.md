# 014 - Never lose a saved reply

## User Story

Diane had a huge day yesterday — 90 replies, all of them saved by slix. This morning she runs slix again and only 10 of yesterday's replies come back. She opens the report and yesterday still reads 90 replies, every single one still there with the numbers it already had. Relieved, she closes the laptop and goes back to sleep.

## Acceptance Criteria

- A reply Diane has already seen never disappears — not from yesterday, not from any older day, and not from today either (even if she passes 100 replies in one day)
- The same reply never appears twice in a day's list, no matter how many times she runs slix
- When a reply does come back in a run, its fresh numbers replace the saved ones — even if the fresh numbers are lower
- A reply that doesn't come back keeps the numbers it already had
- Demo: she runs slix twice in a row and yesterday's reply count doesn't drop between the two runs
- A unit test covers this behaviour

## Technical Design

### Root cause

`save_replies` (src/main.rs:75-89) buckets the fetched replies by local day and calls
`History::save` for each bucket. `History::save` (src/history.rs:80-89) serializes the slice and
renames it over `{date}.json` — the previous contents are discarded, because `load` is never called
in the write path. A day that is present in the fetch but only partially covered (the run fetches at
most `max_results=100`, single page) gets its file rewritten with just that subset. Days absent from
the fetch are untouched, which is why the loss is silent and partial.

### Components and responsibilities

| Component | Knows | Does |
|---|---|---|
| `merge(saved, fresh) -> Vec<Reply>` — new, pure, in `src/history.rs` | nothing (no state) | reconciles two lists by `id` |
| `History` (src/history.rs) | the history dir, the on-disk format | `save` loads the existing day file, calls `merge`, writes the result atomically |
| `save_replies` (src/main.rs:75-89) | how to bucket replies by local day | unchanged — still groups and calls `History::save` per day |
| report / `TodayGoal` | nothing new | unchanged — they read files via `load` / `last_30_day_counts`, which already see the merged result |

### Decisions

1. **The merge lives inside `History::save`.** Saving a day becomes a read-modify-write, so no call
   site can overwrite by accident. `save_replies` stays a dumb grouper. Signature is unchanged:
   `save(&self, date: NaiveDate, replies: &[Reply]) -> Result<(), HistoryError>`.
2. **Match on `Reply::id`; replace the whole record.** The fresh record wins in its entirety —
   numbers included, even when they are lower than the saved ones. No field-level merging.
3. **Order: saved first, then new.** Saved replies keep their existing order; fresh replies whose id
   is not yet saved are appended in fresh order.
4. **A corrupt existing file aborts the save.** `load` returning `Err(Json)` propagates out of
   `save`; we never write over a file we could not read.
5. **`merge` is a pure function**, so the unit test runs without filesystem I/O.

### Test changes

- New unit test on `merge` in `src/history.rs` (no temp dirs) covering all four clauses: nothing
  disappears, no duplicate ids, fresh replaces even with lower numbers, a reply absent from `fresh`
  keeps its saved record.
- Delete `save_replies_called_twice_for_same_day_replaces_the_file_outright`
  (src/main.rs:877-904) — it encodes the contract this spec reverses.
- Existing tests that save over an empty/missing day (`save_then_load_round_trips_replies_for_a_date`,
  `saving_empty_list_round_trips_to_empty_vec`, `last_30_day_counts_*`) keep passing: merging into a
  missing file is just the fresh list.

This supersedes spec 011's "No dedup check — always override" decision
(docs/specs/archive/011-save-every-reply-slix-sees.md).
