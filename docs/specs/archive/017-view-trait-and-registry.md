# 017 - View trait and registry

## Technical Design

Purely technical — no user story. The four report sections are free functions
in `src/report.rs` that `write_report` (src/main.rs:39-67) stitches together by
hand, interleaving blank lines, `---` dividers and the `{handle} · connected`
line between the renderer calls. Introducing a `View` trait plus an `all()`
registry turns `write_report` into a loop and gives every section its own
testable unit.

### `src/view.rs`

New module, declared next to the others in `main.rs`:

```rust
pub trait View {
    fn title(&self) -> &'static str;
    fn render(&self, data: &Dataset, now: DateTime<Local>) -> String;
}

pub fn all() -> Vec<Box<dyn View>> {
    vec![
        Box::new(GoalGrid),
        Box::new(Welcome),
        Box::new(Yesterday),
        Box::new(TopAccounts),
        Box::new(DailyGoal),
    ]
}
```

- The trait stays object-safe: `render` takes one concrete, non-generic
  `Dataset`, so `all()` can hand back a mixed `Vec<Box<dyn View>>`. A
  `Dataset<T>` with per-view input types would force enum dispatch instead.
- Views are unit structs. Each `impl View` supplies its own title literal —
  the view knows its title, nothing passes it in. `title()` never sees data.
- `all()` takes no arguments and is the single owner of section order
  (grid → welcome → yesterday → accounts → today).

| View | `title()` | `render()` body |
|---|---|---|
| `GoalGrid` | `"Last 30 days"` | the squares line (grid minus its title line) |
| `Welcome` | `"Welcome"` | `format!("{} · connected", data.handle)` |
| `Yesterday` | `""` | former `render` verbatim, including its `"Yesterday · {n} replies"` header |
| `TopAccounts` | `"Accounts"` | rows, or `"No data yet."` (former `render_accounts` minus its header) |
| `DailyGoal` | `""` | `"Today: {n} of 5 replies …"` verbatim (former `render_today`) |

- An empty title means `write_report` writes no title line. `Yesterday` and
  `DailyGoal` build their headers from data, and `title()` cannot see data, so
  those headers are the first body line instead.
- `Yesterday` owns the impressions/newest sort currently inside
  `report::render` (src/report.rs:38-44); it gets its timezone from
  `now.timezone()` (`now: DateTime<Local>`), so `Dataset` carries no `tz`.

### `Dataset`

One concrete struct that *owns* its data (no lifetime parameter on `View`),
built as an inline struct literal in `write_report` — no constructor logic,
since it is a plain aggregate:

```rust
pub struct Dataset {
    pub handle: String,
    pub counts: Vec<u64>,
    pub yesterday: Vec<crate::model::Reply>,
    pub ranks: Vec<crate::accounts::AccountRank>,
    pub today_count: u64,
}
```

Views hold no state and never touch `History` or the API — `Dataset` is their
only collaborator.

### `write_report`

Keeps its signature. It loads what it already loads (`last_30_day_counts`,
yesterday's replies, `load_all` + `accounts::rank`, today's count), packs the
parts plus `me.handle` into `Dataset`, then:

```rust
for (i, view) in view::all().iter().enumerate() {
    if i > 0 {
        writeln!(output)?;
        writeln!(output, "---")?;
    }
    let title = view.title();
    if !title.is_empty() {
        writeln!(output, "{title}")?;
    }
    writeln!(output, "{}", view.render(&dataset, now))?;
}
```

- Dividers become uniform for every section pair; the previous uneven spacing
  around the handle line is dropped.
- The report gains a `"Welcome"` title line above `{handle} · connected`.
  Output shape is explicitly *not* frozen — only the content the four
  renderers produce today has to survive.

### `report.rs`

- Deleted: `render_grid`, `render`, `render_accounts`, `render_today`,
  `GRID_TITLE`. `DAILY_GOAL` moves to `view.rs` (both `GoalGrid` and
  `DailyGoal` need it).
- Stays as formatting primitives, widened to `pub(crate)` for `view.rs`:
  `Widths` + `render_line`, `AccountWidths` + `render_account_line`,
  `preview`, `link`, `account_link`, `PREVIEW_*`.
- Section composition (header, sorting, joining rows) moves into the view
  impls; `report.rs` only formats a single row.

### Test changes

No assertion may be satisfied by a constant or by calling the code under test
(`lines[15] == report::render_today(0)`, `lines[0] == GRID_TITLE`) — every
expectation is a literal written in the test.

Assembly tests in `src/main.rs` split the report on `"\n\n---\n"` and assert
each chunk against a hand-written literal multi-line string; section order is
asserted by chunk index.

| Test | Change |
|---|---|
| `connected_path_prints_report_after_handle` (src/main.rs:488) | Split on `"\n\n---\n"`, assert all five chunks as literals; drop `report::render_today(0)`. |
| `report_starts_with_thirty_day_grid_then_divider_then_handle` (src/main.rs:542) | Chunk 0 is the literal `Last 30 days` + squares line; drop `report::GRID_TITLE` and `report::render_grid`. |
| `empty_day_prints_no_replies_message` (src/main.rs:581) | `ends_with(render_today(0))` → literal tail `"Today: 0 of 5 replies (5 to go)\n"`. |
| `accounts_section_appears_between_history_and_today_with_dividers` (src/main.rs:608) | Split on `"\n\n---\n"`, literal chunks; drop `report::render_today(0)`. |
| `accounts_section_shows_no_data_yet_when_history_empty` (src/main.rs:675) | Split on `"\n\n---\n"`, literal chunks; drop `report::render_today(0)`. |
| `contains(&report::render_today(1))` (src/main.rs:741, :946) | Literal `assert!(text.contains("Today: 1 of 5 replies (4 to go)"))`. |
| `verify_on_startup_prints_handle_without_oauth` (assert at src/main.rs:367) and `re_open_with_persisted_tokens_stays_connected_without_oauth` (assert at src/main.rs:453) | These only care that the handle line is printed, not where it sits: replace `text.lines().nth(4)` with a literal `assert!(text.contains(&format!("@{username} · connected")))`. |
| Section tests in `src/report.rs` that render a whole section (`shows_local_time_handle_and_impressions`, `aligns_columns_across_replies`, `lists_*`, `starts_with_header_counting_replies`, `says_so_when_there_are_no_replies`, `shows_*_daily_goal`, `grid_*`, `shows_filled_square_*`, `shows_dot_*`, `joins_multiple_days_*`, accounts tests at src/report.rs:460-521) | Move to `view.rs` `mod tests`, driving `view.title()` and `view.render(&dataset, now)` on a `Dataset` fixture with a fixed `now`. `grid_has_title_line_above_the_squares` becomes `assert_eq!(GoalGrid.title(), "Last 30 days")` plus a literal squares body; `render_today` expectations become full literals (`"Today: 3 of 5 replies (2 to go)"`); `grid_squares(&[DAILY_GOAL])` uses `5`, not the constant. |
| Helper tests in `src/report.rs` (preview, OSC-8 links, per-field rows, `aligns_columns_*` line formatting) | Stay in `report.rs`; only the expectations that interpolate `DAILY_GOAL` become literals. |
