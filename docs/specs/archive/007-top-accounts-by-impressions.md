# 007 - Top accounts by impressions

## User Story

As Slix, when I run the report, I want to see a ranked list of the top 10 accounts I've replied to across my whole history, by average impressions per reply, so I know which accounts to reply to again.

## Acceptance Criteria

- A new "Accounts" section appears in the report, positioned after the History section and before the Today section, with dividers separating it from both.
- The section lists up to the top 10 distinct accounts (handles) I've replied to, across my whole reply history, ranked by average impressions per reply, highest first.
- Each line shows: rank, handle (linked to the account's profile), average impressions (rounded to the nearest whole number), and the number of replies that average is based on.
- Average impressions is the mean of impressions across all my replies to that handle.
- If I've replied to fewer than 10 distinct accounts, the section lists all of them.
- If there's no reply history yet, the section says "No data yet."

## Technical Design

### `history.rs`

- Add `History::load_all() -> Result<Vec<Reply>, HistoryError>`. Lists every date file in the history directory, sorted by date ascending, and concatenates their replies. Propagates `HistoryError` if any file fails to parse — same failure behavior as `load`, no partial/skip-on-corrupt handling.

### `accounts.rs` (new module)

- `pub struct AccountRank { pub handle: String, pub avg_impressions: f64, pub reply_count: usize }`
- `pub fn rank(replies: &[Reply]) -> Vec<AccountRank>`:
  - Groups replies by handle, normalizing case (lowercased) only for the grouping key; the displayed `handle` keeps the casing of the first reply seen for that account.
  - Computes average impressions (mean, left unrounded) and reply count per group.
  - Sorts by average impressions descending using a stable sort, so accounts tied on average impressions keep their original (first-seen-in-input) relative order.
  - Truncates to the top 10.
- Owns all grouping/ranking/truncation logic; does no formatting.

### `report.rs`

- `pub fn render_accounts(ranks: &[accounts::AccountRank]) -> String`:
  - Header line: `Accounts`.
  - If `ranks` is empty, appends `No data yet.`.
  - Otherwise renders one line per entry, aligned into columns the same way the History section uses its `Widths` struct: rank, handle, average impressions (rounded to nearest whole number), reply count.
  - The handle itself is the OSC8 hyperlink target (`https://x.com/{handle}`), not a separate `[link]` element.
  - Takes no timezone parameter — there are no timestamps to render.

### `main.rs` / `write_report`

- After the History section and before the Today section: call `history.load_all()`, pass the replies to `accounts::rank`, and render the result with `report::render_accounts`.
- Wrap the Accounts section with the divider convention: blank line, `---`, blank line — both before and after the section.
- Recomputed on every run (no caching), since it's a pure local read of already-saved history files rather than an API call.
