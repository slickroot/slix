# 010 - One fetch for latest 100 posts

## User Story

Maro opens slix in the morning. slix asks X once for Maro's latest 100 posts, and both the "Yesterday" section and the "Today: X of 5" line are filled from that one answer. Maro checks the X dashboard and sees one request for that run instead of two, while the report looks just like it did before. Happy, Maro starts replying.

## Acceptance Criteria

- Each run makes exactly one request to X for posts
- The report shows the same Yesterday and Today numbers as the current version

## Technical Design

**Scope constraint:** one `GET /2/users/:id/tweets` request per run, down from up to two. The endpoint and query stay the same except for the time window.

### Decisions

- **Endpoint:** stay on the user timeline. Its `exclude` only accepts `replies` and `retweets`, so original posts and replies to self are still fetched and then dropped by the existing `replied_to` filter. That's fine for this story.
- **No time window in the request:** the API client asks for the latest 100 posts. Splitting into days happens locally with `Window`, which already handles local midnight and DST.
- **Fetch on every run:** `write_report` fetches once at the start, whatever the caches hold. X doesn't bill again for posts fetched earlier in the same UTC day (00:00 UTC reset), so the extra request on a fully cached run costs at most one batch per UTC day. It also removes all branching on the caches.
- **Caches stay the same:** Yesterday still loads from `history` when saved, and Today still loads from `today_goal` when fresh. The fetched posts are used only to fill what's missing. Rethinking storage belongs to specs 011–013.

### Components

1. **`api`** (changed)
   - `replies(user_id, window)` becomes `latest_replies(user_id) -> Result<Vec<Reply>, ApiError>`.
   - The query drops `start_time` and `end_time`. `max_results=100`, `exclude=retweets`, `tweet.fields` and `expansions` are unchanged.
   - The `replied_to` filtering, username resolution and error mapping are unchanged. `api` no longer depends on `window`.

2. **`window`** (extended)
   - `Window::contains(&self, at: &DateTime<Utc>) -> bool`: `start <= at < end`.

3. **`main::write_report`** (changed)
   - `let latest = api.latest_replies(&me.id)?;` runs once, before any section.
   - Today: `today_goal.load(now)?`, or otherwise count the `latest` replies where `Window::today(now).contains(&reply.created_at)`, then `today_goal.save`.
   - Yesterday: `history.load(date)?`, or otherwise the `latest` replies where `Window::yesterday(now).contains(&reply.created_at)`, then `history.save`.
   - Ownership: compute the Today count by reference first, then consume `latest` with `into_iter().filter(..).collect()` for Yesterday. This way `Reply` doesn't need `Clone`. Output order is unchanged: Yesterday, Accounts, Today.

`history`, `today`, `report` and `accounts` are untouched.

### Implementation order (TDD)

1. `window`: `contains` is true at `start`, false at `end`, and a moment exactly at local midnight belongs to today, not yesterday.
2. `api`: rename `replies_sends_signed_get_with_window_and_field_params` to `latest_replies_sends_signed_get_without_time_window`. Assert that `start_time`/`end_time` are absent and the other params are unchanged. Rename the other `replies_*` tests to `latest_replies_*`, with no change to their behaviour.
3. `main`: acceptance test. With history and `today_goal` both empty, one tweets mock returns one reply from yesterday and one from today (timestamps built from `now()`). Assert `tweets.calls() == 1`, the Yesterday section lists yesterday's reply, and the Today line is `render_today(1)`.
4. `main`: rewrite `cached_fresh_today_count_is_reused_without_calling_api` as `cached_fresh_today_count_is_reused_over_fetched_posts`. Assert `tweets.calls() == 1` and that the cached count is shown, not the fetched one.
5. `main`: update the remaining `write_report` tests (`first_run_of_day_saves_fetched_replies_to_history`, `no_cached_today_count_...`, etc.) to expect exactly one tweets call per run.
