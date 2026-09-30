# 018 - One column-width implementation in ui::table

## Technical Design

Purely technical — no user story. Column widths are computed twice today, in two
bespoke structs in `src/report.rs`, each with its own four `max()` calls and its
own hand-written padding in a `format!` template:

- `Widths { handle, impressions, likes, profile_visits }` + `Widths::of` +
  `render_line` (src/report.rs:8-60)
- `AccountWidths { rank, handle, avg_impressions, reply_count }` +
  `AccountWidths::of` + `render_account_line` (src/report.rs:76-122)

`render_account_line` cannot use a width specifier at all: its `@handle` cell is
wrapped in OSC-8 escapes, so it pads by hand with
`" ".repeat(widths.handle.saturating_sub(handle_text.chars().count()))`
(src/report.rs:115). The duplication is the bug — the second one exists only
because the first one was not general.

`src/report.rs` becomes `src/ui/`, and the column-width math moves into a
domain-free `ui::table` that renders a whole block of rows from a declared
layout.

### `src/ui/table.rs`

New module, declared as `pub(crate) mod table;` in `src/ui/mod.rs`. It knows
about strings, columns and escapes — not about `Reply`, `AccountRank` or
terminals.

```rust
pub(crate) struct Column {
    align: Align,
    width: Width,
}

impl Column {
    pub(crate) fn left() -> Column;
    pub(crate) fn right() -> Column;
    pub(crate) fn fixed(self, n: usize) -> Column;
}

pub(crate) struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub(crate) fn new(columns: Vec<Column>) -> Table;
    pub(crate) fn rows(self, rows: impl IntoIterator<Item = Vec<String>>) -> Table;
    pub(crate) fn render(&self) -> String;
}
```

`Align` (`Left`/`Right`) and `Width` (`Auto`/`Fixed(n)`) are private; the three
constructors are the whole API. `Column::fixed` is a modifier so alignment and
width stay orthogonal: the preview is `left().fixed(PREVIEW_WIDTH)`, and no
numeric column is fixed.

Rules, all of them table-wide with no per-column exceptions:

- **One gap of two spaces between every pair of adjacent columns.** No
  number-to-label special case.
- **Alignment and width rule are column properties**, so they are identical on
  every row by construction. A row supplies text only and cannot disagree with
  the row above it.
- `Width::Auto` is the max of `display_width` over that column's cells, or `0`
  when there are no rows.
- `Width::Fixed(n)` is `n`. A cell wider than `n` is emitted whole and not
  padded — the table has no notion of an ellipsis; `ui::preview` owns
  truncation.
- **The last cell of a row is never padded**, so no row ends in whitespace.
- A row whose arity differs from the layout is a bug: `assert_eq!` naming the
  row index. This is a view-authoring error, not a data error.
- Zero rows renders `""`. Both views still return early on empty data, so this
  only exists to make the type total.

`display_width` is what makes escaped cells measurable:

```rust
pub(crate) fn strip_escapes(s: &str) -> String;
fn display_width(s: &str) -> usize;  // strip_escapes(s).chars().count()
```

`strip_escapes` walks the bytes once. On `ESC`, a `]` starts an OSC string that
runs to its `ESC \` (or BEL) terminator and is dropped whole; a `[` starts a CSI
sequence that runs to its final byte (`0x40..=0x7e`) and is dropped. This is
the *only* escape grammar in the crate — `view.rs`'s `strip_osc8` test helper
(src/view.rs:203) is deleted in favour of it.

`display_width` counts `chars()`, matching today's `{:<PREVIEW_WIDTH$}`
behaviour; only the byte count of an escaped cell was ever wrong.

### `src/ui/mod.rs` (was `src/report.rs`)

Owns the column layout and the row builder for each of the two tables, so
`view.rs` never mentions columns, alignment or widths.

```rust
pub(crate) fn yesterday_table<Tz>(replies: &[&Reply], tz: &Tz) -> String;
pub(crate) fn accounts_table(ranks: &[accounts::AccountRank]) -> String;
```

`yesterday_table` layout, ten columns:

| # | cell | column |
|---|---|---|
| 0 | `%H:%M` local time | `left()` |
| 1 | `@username` | `left()` |
| 2 | `preview(&reply.text)` | `left().fixed(PREVIEW_WIDTH)` |
| 3 | impressions | `right()` |
| 4 | `impressions` | `left()` |
| 5 | likes | `right()` |
| 6 | `likes` | `left()` |
| 7 | profile visits | `right()` |
| 8 | `profile visits` | `left()` |
| 9 | `link(&reply.id)` | `left()` |

`accounts_table` layout, six columns:

| # | cell | column |
|---|---|---|
| 0 | `format!("{rank}.")` | `right()` |
| 1 | `account_link(&account.handle)` | `left()` |
| 2 | rounded `avg_impressions` | `right()` |
| 3 | `avg impressions` | `left()` |
| 4 | `reply_count` | `right()` |
| 5 | `replies` | `left()` |

- **A number and its label are two columns, not one cell.** Their labels differ
  in length (`impressions` 11, `likes` 5, `profile visits` 14), so no single
  left/right-aligned cell can hold both. This is what forces the uniform gap.
- **The rank cell is `"{rank}."`, right-aligned.** The padding lands to the
  left of the dot, which is the same visible result as today's
  `{rank:>rank_width$}. ` — the dot is a cell, not a gap. The rank column is
  auto-width off `display_width("1.")`, so a 10-row table pads to `3`.
- `preview`, `link`, `account_link`, `PREVIEW_LIMIT` and `PREVIEW_WIDTH` move
  here unchanged and keep their tests.

### `view.rs`

The views own composition; `ui` owns layout.

- `Yesterday::render` keeps the impressions/newest sort, the
  `Yesterday · {n} replies` header and the empty case, then returns
  `ui::yesterday_table(&ranked, &tz)`.
- `TopAccounts::render` keeps the empty case, then returns
  `ui::accounts_table(&data.ranks)`.

### Deleted

`Widths`, `Widths::of`, `render_line`, `AccountWidths`, `AccountWidths::of`,
`render_account_line`, and with them both `format!` templates and the manual
`" ".repeat(...)` padding. Padding is no longer spelled out per row; that is
the entire point of the refactoring. `render` keeps only `preview`, `link`,
`account_link`, the two `*_table` functions and the two `PREVIEW_*` constants.

### Output change (deliberate, and the only one)

The number-to-label gap goes from one space to two, and the rank-to-handle gap
from one space to two. Cell content, ordering and column widths are unchanged.
The literals that pin it:

| Test | Was | Becomes |
|---|---|---|
| `aligns_columns_across_replies` (src/view.rs:262) | `"@bobby  {:<51$}  1234 impressions  1234 likes  1234 profile visits"` | `"@bobby  {:<51$}  1234  impressions  1234  likes  1234  profile visits"` |
| `aligns_columns_across_replies` (src/view.rs:267) | `"@al     {:<51$}     5 impressions     5 likes     5 profile visits"` | `"@al     {:<51$}     5  impressions     5  likes     5  profile visits"` |
| `aligns_columns_across_accounts` (src/view.rs:427) | `"1. @al        5 avg impressions     5 replies"` | `"1.  @al        5  avg impressions     5  replies"` |
| `aligns_columns_across_accounts` (src/view.rs:428) | `"2. @bobby  1234 avg impressions  1234 replies"` | `"2.  @bobby  1234  avg impressions  1234  replies"` |
| `connected_path_prints_report_after_handle` (src/main.rs:533) | `12 impressions  3 likes  4 profile visits` | `12  impressions  3  likes  4  profile visits` |
| `accounts_section_appears_between_history_and_today_with_dividers` (src/main.rs:539) | `"1. \x1b]8;;…\x1b\\@bob\x1b]8;;\x1b\\  12 avg impressions  1 replies"` | `"1.  \x1b]8;;…\x1b\\@bob\x1b]8;;\x1b\\  12  avg impressions  1  replies"` |
| `connected_path_prints_report_after_handle` (src/main.rs:672) | as :533 | as :533 |
| `accounts_section_…with_dividers` (src/main.rs:678) | `1. \x1b]8;;…alice…\x1b\\  20 avg impressions  1 replies\n2. \x1b]8;;…bob…\x1b\\    12 avg impressions  1 replies` | `1.  …alice…\x1b\\  20  avg impressions  1  replies\n2.  …bob…\x1b\\    12  avg impressions  1  replies` |

### Test plan

No assertion may be satisfied by calling the code under test. Every table
expectation is a hand-written literal, including the spaces.

`src/ui/table.rs` `mod tests`:

- puts two spaces between every adjacent pair of columns, including between a
  number and its label
- right-aligns a numeric column so its widest cell defines the width
- left-aligns and pads a column to its widest cell
- pads a `fixed` column to its fixed width no matter what the other rows hold
- emits a cell wider than its fixed width whole, with no truncation and no
  ellipsis
- measures a cell by visible text: an OSC-8 hyperlink column aligns on the
  visible handle, so `@bobby` and `@al` line up despite different byte lengths
- never pads the last cell, so no row ends in whitespace
- keeps a number column and a label column aligned when a later row has a
  shorter label
- renders `""` for zero rows
- `#[should_panic]` when a row's arity does not match the layout
- `strip_escapes` drops an OSC-8 hyperlink, including its `ESC \` terminator,
  and a CSI colour sequence, keeping the text between them

`src/ui/mod.rs` `mod tests`: today's `report.rs` tests move over unchanged
(`collapses_newlines_in_preview`, `keeps_preview_at_the_limit`,
`cuts_preview_over_the_limit_with_ellipsis`, `links_with_osc8_hyperlink`,
`shows_zero_likes`, `uses_the_word_likes_for_a_single_like`,
`puts_likes_between_impressions_and_link` and the profile-visits trio) — they
are about cell content, not layout, and none of their assertions mention a
width. Add two row-shape tests: `yesterday_table` puts the link in the last
column, `accounts_table` renders the rank with its dot.

`src/view.rs` `mod tests`: the two `aligns_columns_*` literals become the new
ones above; `strip_osc8` is deleted in favour of `ui::table::strip_escapes`.

`src/main.rs` `mod tests`: the four chunk literals at :533, :539, :672, :678
change gaps only. No other assembly assertion touches a table row.

### Order of work

1. `ui::table` with its tests, and `strip_escapes` — nothing calls it yet.
2. `accounts_table` behind `TopAccounts`, driven by the updated
   `aligns_columns_across_accounts` and main.rs literals. This is the simpler
   table: six columns, no fixed width, and the OSC-8 handle is the case that
   proves `display_width` is needed.
3. `yesterday_table` behind `Yesterday`, driven by the updated
   `aligns_columns_across_replies` and main.rs literals. Adds the `fixed`
   width rule.
4. Delete `Widths`, `AccountWidths`, `render_line` and `render_account_line`;
   rename the module to `ui` in `main.rs`.
