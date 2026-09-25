# 013 - Thirty-day goal grid

## User Story

Maro opens slix. At the very top of the report, a single row of 30 symbols shows the last 30 days, oldest on the left and today on the right. `■` means Maro hit the goal of 5 replies that day and `□` means Maro missed it. Maro spots a streak of `■` at the end of the row and, happy, keeps it going.

## Acceptance Criteria

- The grid is the first section of the report
- It's a single row of 30 days, oldest on the left and today on the right
- A day with 5 or more saved replies shows `■`
- A day with fewer than 5 saved replies, including none, shows `□`
- Today counts too, based on the replies saved so far

## Technical Design

- `History` gains `pub fn last_30_day_counts(&self, today: NaiveDate) -> Result<Vec<u64>, HistoryError>`. It loops the 30 dates ending on `today` (oldest first) and, for each, computes `self.load(date)?.unwrap_or_default().len()`. `History` has no clock of its own, so `today` is passed in by the caller, matching how `load`/`save` are already given dates from outside.
- `report.rs` gains `pub fn render_grid(counts: &[u64]) -> String`, next to `render_today`. It maps each count to `■` (count >= `DAILY_GOAL`) or `□` (otherwise), reusing the existing `DAILY_GOAL` constant, and joins them into a single line with no separators.
- `write_report` (in `main.rs`) calls `history.last_30_day_counts(now.date_naive())?` and writes `report::render_grid(&counts)` as the first line of the report, followed by a blank line and a `---` divider before the existing handle line.
