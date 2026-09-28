# 015 - Order yesterday's replies by impressions

## User Story

Diane wrote 8 replies yesterday. She runs the report and yesterday's list now opens with her best-performing reply — the highest impression count right at the top — so she instantly sees which one landed. Smiling, she closes the laptop.

## Acceptance Criteria

- Yesterday's list shows replies with the highest impressions first
- When two replies have the same impressions, the newer one is shown first
- Only the Yesterday list is affected
- Demo: run the report and watch yesterday's list come out with the biggest impression counts at the top

## Technical Design

### Ordering responsibility

Ordering stays inside `report::render` (src/report.rs:38-39) — no new component. `render` is
called from exactly one site, `write_report` (src/main.rs:59), which is the Yesterday section;
the grid, accounts, and today sections each have their own render functions, so the
"only the Yesterday list is affected" criterion holds by construction.

### Sort key

Replace the current key with a compound tuple key:

```rust
let mut newest_first: Vec<&Reply> = replies.iter().collect();
newest_first.sort_by_key(|reply| (std::cmp::Reverse(reply.impressions), std::cmp::Reverse(reply.created_at)));
```

- Primary: `impressions` descending (highest first)
- Secondary: `created_at` descending (newer first on ties)

Both fields are `Ord` (`u64`, `DateTime<Utc>`), so the tuple key resolves ties by construction
and needs no explicit comparison function or stable-sort reliance.

Column widths (`Widths::of`, src/report.rs:57-82) are computed as maxima and are therefore
order-independent; reordering does not affect alignment.

### Test changes

| Test | Change |
|---|---|
| `lists_highest_impressions_first` (new) | Fixtures with distinct impression counts where the higher impression reply is the older one; asserts it appears first. This is the red step. |
| `lists_newest_first` → `lists_newest_first_when_impressions_tie` (rename) | Fixtures already share `impressions: 1`, so it passes unchanged and now documents the tie-break rule. |
| `aligns_columns_across_replies` (src/report.rs:319) | Swap expected lines: `@bobby` (1234 impressions) now sorts before `@al` (5). |

No new integration test in `src/main.rs`: ordering exists only in `report::render`, which has a
single caller, so the unit tests are not duplicated end-to-end.
