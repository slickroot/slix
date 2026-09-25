# 012 - Refresh numbers on saved replies

## User Story

On Monday, Maro replied to @alice, and the reply had 40 impressions when slix first saved it. The reply keeps getting views. On Thursday, Maro opens slix, and the saved reply's impressions, likes and profile visits are updated to X's latest numbers. Maro looks at the Accounts section and sees @alice's average impressions go up. Happy, Maro knows the ranking reflects how replies really did.

## Acceptance Criteria

- When a saved reply shows up in the fetch, its impressions, likes and profile visits are updated to the new numbers
- If X leaves out one of those numbers, slix keeps the one it already has
- The Accounts averages use the updated numbers

## Technical Design

**Decision: no implementation needed. Archived as already satisfied (mostly) as a side effect of spec 011, with the one gap deliberately dropped.**

### What spec 011 already gives us

`save_replies` (spec 011, `src/main.rs`) groups every fetched reply by local posted day and calls `history.save(date, replies_for_that_day)`, which **overwrites that day's file outright** with whatever the current fetch returned — no load-and-merge. Since spec 010 fetches the latest 100 posts on every run, any saved reply that's still in that window gets re-fetched and its file rewritten with X's latest numbers.

- **AC1** ("saved reply's impressions/likes/profile visits are updated to the new numbers") — satisfied. Proven by the existing test `save_replies_called_twice_for_same_day_replaces_the_file_outright` (impressions 5 → 99 on a repeat save of the same id).
- **AC3** ("Accounts averages use the updated numbers") — satisfied. `accounts::rank` reads from `history.load_all()`, which reflects whatever `save_replies` last wrote.

### What's not covered, and why that's fine

- **AC2** ("If X leaves out one of those numbers, slix keeps the one it already has") — not satisfied, and not being pursued. `PublicMetrics`/`NonPublicMetrics` in `src/api.rs` deserialize `impression_count`, `like_count`, `user_profile_clicks` as required `u64` fields; a response missing any of them fails the whole `latest_replies` call with `ApiError::Protocol`, which aborts the entire run rather than falling back per-field. Decided not worth building: not interesting enough to chase on its own.
- Replies that have fallen out of the latest-100 window no longer get refreshed — accepted as an inherent limit of spec 010's fetch, not something this spec was trying to fix.

No code changes made for this spec.