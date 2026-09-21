# 002 - Maro sees yesterday's replies with their impressions

Maro opens Slix and sees the replies he made yesterday, each with its impressions, so that he can tell which replies got seen.

## Acceptance Criteria

1. Maro sees the replies he made yesterday, meaning the previous calendar day in his own time zone (midnight to midnight).
2. Each reply shows:
   - its time
   - a preview of the reply text
   - the account it replied to
   - its impressions
   - a clickable `[link]` to the reply on X that works in his terminal
3. The replies are listed newest first.
4. If Maro made no replies yesterday, he sees a short message saying so.

## Technical Design

**Scope constraint:** still read-only against X. Only GET calls are added.

### Decisions

- **Impressions:** `public_metrics.impression_count`. Works with the existing OAuth 1.0a user context and matches the number visible on X. Labelled `impressions` in the output.
- **Time zone:** OS local zone, no config option. `chrono` is added as a dependency.
- **Fetching:** one request to `GET /2/users/:id/tweets`, `max_results=100`, **no pagination**. Replies beyond the first 100 tweets of the window are not shown (accepted limit for v1).
  - Params: `start_time`, `end_time` from the window, `exclude=retweets`, `tweet.fields=created_at,public_metrics,referenced_tweets,in_reply_to_user_id`, `expansions=in_reply_to_user_id`.
  - The API has no "replies only" filter, so a tweet is kept iff `referenced_tweets` contains a `replied_to` entry. The replied-to `@username` comes from `includes.users`.
- **Link:** OSC 8 terminal hyperlink around the text `[link]`, pointing to `https://x.com/i/status/{id}` (needs only the tweet id). Terminals without OSC 8 show a plain `[link]`.
- **Errors:** a failed reply fetch propagates as an error, like `users_me` after the PIN flow. 401/403 map to `NeedsReconnect` as today.

### Components

1. **`window`** (new): `Window { start, end }`, both UTC instants.
   - `Window::yesterday(now: DateTime<Local>) -> Window`: local midnight yesterday up to local midnight today. Pure: no clock or zone reads, so DST edges are testable with a fixed `now`.

2. **`api`** (extended): `XApiClient`
   - `users_me()` now returns `Me { id, handle }` instead of only the handle.
   - `replies(user_id, &Window) -> Result<Vec<Reply>, ApiError>`.
   - `Reply { id, created_at: DateTime<Utc>, text, to_username, impressions }`.
   - A response missing required fields maps to `ApiError::Protocol`.

3. **`report`** (new): `render(&[Reply], tz) -> String`, pure.
   - Newest first. One line per reply: local `HH:MM`, `@to_username`, preview, impressions, `[link]`.
   - Preview: newlines collapsed to spaces, cut at 50 chars with `…` when cut.
   - Columns aligned: handle padded to the widest handle, preview padded to a fixed width, impression counts right-aligned. Padding counts chars, not display width (emoji/CJK may shift a column slightly; accepted for v1).
   - Header line `Yesterday · N replies`.
   - No replies: `No replies yesterday.` instead of the list.

4. **`main::run`** (extended)
   - New parameter `now: DateTime<Local>`; `main` passes `Local::now()`.
   - After `@handle · connected` (verified or freshly connected), builds `Window::yesterday(now)`, calls `api.replies(me.id, &window)`, and writes `report::render(...)`.
   - The silent-reconnect path prints only `reconnecting…` and the authorize URL, with no reply list. Maro re-runs Slix to see replies.

```
             main::run
      ┌────────┼─────────┬──────────┐
    config   oauth      api      window / report
                         │
              reqwest + oauth1 (GET only)
```

### Implementation order (TDD)

1. `window`: `yesterday` for a normal day, a DST-change day, and a `now` just after midnight.
2. `api`: `users_me` returns `Me { id, handle }` (update the existing tests).
3. `api`: `replies` request shape (params, signed GET) against a mock server.
4. `api`: `replies` keeps only `replied_to` tweets, resolves `to_username`, and maps 401/403/500/malformed bodies to errors.
5. `report`: one reply line (time, preview, truncation, OSC 8 link bytes).
6. `report`: alignment across several replies, newest-first order, header count, empty message.
7. `main`: connected path prints the report; empty-day message; the reconnect path prints no report.
