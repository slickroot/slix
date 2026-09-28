# 016 - Separate the model from the X API types

## Refactoring

`api::Reply` is simultaneously three things: what the X API returns, what slix writes to
`history/*.json`, and what `history`, `report` and `accounts` read. If X changes a field, or we
rename one, the stored history breaks and every view breaks with it. There is also dead weight
around it: `today.rs` writes a `today.json` that is a no-op round trip of a number `history`
already holds, `window.rs` is `#[allow(dead_code)]` and referenced by nothing, and `History`
still offers `Default`/`default_dir()` — a second, unused source of truth for the storage path
that `Config::data_dir` already owns.

## Acceptance Criteria

- `cargo test` passes.
- **Every existing report-output assertion in `src/main.rs` is untouched, byte for byte.** Those
  exact-line tests are the proof that nothing user-visible changed. The diff to them must be
  empty.
- The only test deltas are the ones named under *Test changes* below — nothing else is added,
  renamed or deleted.
- `history/*.json` written before this refactor still loads, and files written after it have the
  same field names and values as before.
- `today.json` is never written again; no output line changes.
- `cargo build` carries no module-wide `#[allow(dead_code)]`, and `chrono-tz` is gone from
  `Cargo.toml` and `Cargo.lock`.

## Technical Design

### Components and responsibilities

| Component | Knows | Does |
|---|---|---|
| `model` (new `src/model.rs`) | nothing — it is a leaf: chrono + serde only | owns `Reply` and `Me`; is the domain model *and* the storage format |
| `api` (src/api.rs) | the X wire shape (`Tweet`, `PublicMetrics`, `NonPublicMetrics`, `Includes`, `ReferencedTweet`, `UserMe`, all already private) and OAuth/HTTP | maps wire → `model::Reply` / `model::Me` at the edge; publicly exports **only** `XApiClient` and `ApiError` |
| `history` (src/history.rs) | the history dir, the on-disk format | reads and writes `Vec<model::Reply>`; owns the two format tripwire tests |
| `report`, `accounts` | how to render/rank | unchanged behaviour, `use crate::model::Reply` instead of `use crate::api::Reply` |
| `main` (src/main.rs) | the run order and every collaborator | the only module allowed to import `api`; drops the `today_goal` plumbing |

### Dependencies

```
model  <── api        (api maps wire → model)
model  <── history, report, accounts, main
api    <── main only
```

`model` imports nothing from this crate. `history`, `report` and `accounts` never name `api`.

### Decisions

1. **`src/model.rs` is a new leaf module** holding `Reply` (moved verbatim, still
   `Debug, PartialEq, Serialize, Deserialize`) and `Me` (`Debug` only — it is never stored).
   Declared in `main.rs` alongside the other modules.
2. **`api` maps into the model.** `latest_replies` returns `Vec<model::Reply>`, `users_me`
   returns `model::Me`; the conversion stays inside `api.rs`, so no wire type escapes. The model
   can never be reshaped by an X API change because `api`'s private structs are the only thing
   that knows the wire shape.
3. **`api`'s public surface is exactly `XApiClient` + `ApiError`.** No `pub use` of model types,
   no `api::Reply`, no `api::Me`. "Views never import api" is therefore structural: there is no
   API data type left to import. No source-scanning test — explicitly out of scope.
4. **`Me.handle` keeps its leading `@`** (`format!("@{username}")` stays in `users_me`), so this
   refactor changes zero bytes. The fact that `AccountRank.handle` is bare while `Me.handle` is
   not is documented here as known, not fixed.
5. **The model is the storage format.** `history/*.json` is the serde encoding of
   `Vec<model::Reply>` and nothing else may define that shape.
6. **Today's count comes from `history`, not a cache.** `write_report` computes
   `history.load(now.date_naive())?.unwrap_or_default().len()` for `render_today`. `TodayGoal`'s
   1-hour staleness rule (spec 006) is already inert — `main` wrote the count with the same `now`
   it later read it with — so deleting the round trip is behaviour-preserving. An existing
   `today.json` on disk is orphaned and left alone; slix does not clean it up.
7. **`window.rs` goes, and `chrono-tz` with it** — it is used only by that file's own tests.
8. **`History`'s `Default` impl, `default_dir()` and their two tests go.** `Config::data_dir` is
   the single source of truth for where history lives; `dirs` stays in `[dependencies]` because
   `config.rs` still needs it.

### Test changes

- **New, written first (the tripwire), in `src/history.rs`:**
  - `save_writes_the_pinned_history_format` — `save` `sample_reply("1")` for a date, read the
    file back as a `serde_json::Value`, and assert equality against a `json!` literal naming all
    seven fields (`id`, `created_at`, `text`, `to_username`, `impressions`, `likes`,
    `profile_visits`). Capture the literal from the implementation's actual output — we are
    freezing the existing format, not inventing one.
  - `a_file_written_before_the_split_still_loads` — an inline JSON literal in that exact shape
    must parse into `model::Reply` with every field intact.
  - These go green against today's `api::Reply` and must stay green through the whole split.
- **`src/api.rs`:** delete `reply_round_trips_through_json` — strictly weaker than the golden
  write. The remaining api tests only change their `Reply` import path.
- **`src/main.rs`:**
  - `today_count_is_derived_from_the_fresh_fetch_not_a_stale_cache` →
    `today_count_comes_from_todays_history_file`: drop the `today_goal.save(3, …)` setup and the
    `today_goal` argument; keep the `render_today(1)` assertion.
  - `fetch_spanning_a_skipped_day_saves_every_reply_under_its_own_day`: drop the `today_goal`
    construction/argument and replace `today_goal.load(now) == Some(1)` with the equivalent
    assertion against `history`/the rendered report.
  - Delete the `test_today_goal` helper and the `today_goal` parameter from
    `connect_and_write_report` and `write_report` at every call site.
  - All report-output assertions (line arrays, `render_today(0)`, grid lines) are **not** edited.
- **`src/history.rs`:** delete `default_dir_is_config_dir_joined_with_slix_history` and
  `default_uses_default_dir`.

### Implementation order (TDD)

1. **Tripwire:** add the two format tests to `src/history.rs` — green immediately, freezing the
   current shape of `history/*.json`.
2. **Split:** create `src/model.rs`, move `Reply` + `Me`, repoint `history`/`report`/`accounts`
   at `crate::model`, make `api` return model types and export only `XApiClient`/`ApiError`.
   The tripwire must not go red.
3. **Delete `today.rs`, test first:** rewrite the two named `main.rs` tests and drop the
   `today_goal` parameter everywhere (red — it no longer compiles/behaves), then delete the
   module, the `mod today;` line and the `save` call (green).
4. **Delete `window.rs`**, the `mod window;` line and `chrono-tz`.
5. **Delete `History`'s `Default`/`default_dir`** and their two tests.
