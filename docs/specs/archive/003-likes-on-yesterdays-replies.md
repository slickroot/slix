# 003 - Maro sees likes on yesterday's replies

Maro opens Slix and sees how many likes each of yesterday's replies got, so that he can tell which replies people responded to.

## Acceptance Criteria

1. Each reply in yesterday's report shows its likes, after the impressions and before the `[link]`, for example: `14:32  @alice  Great point about the…  340 impressions  12 likes  [link]`
2. A reply with no likes shows `0 likes`.
3. A reply with one like shows `1 likes`. The word is always `likes`.
4. The like counts are right-aligned, so they line up in the same column across replies.

## Technical Design

**Scope constraint:** no new requests. Likes ride on the existing `GET /2/users/:id/tweets` call.

### Decisions

- **Source:** `public_metrics.like_count`. `public_metrics` is already in `tweet.fields`, so the request does not change.
- **Missing value:** `like_count` is required, like `impression_count`. A reply without it maps to `ApiError::Protocol` and the report fails. No silent `0 likes`, because it would look like real data.
- **Label:** always `likes`, including for 0 and 1 (no pluralisation logic).
- **Refactor:** `render_line` takes a private `Widths` struct instead of one positional width per column. Spec 004 (profile visits) then adds a single field.

### Components

1. **`api`** (extended)
   - `PublicMetrics` gains `like_count: u64`, required.
   - `Reply` gains `likes: u64`, mapped from `like_count`.

2. **`report`** (extended)
   - Private `Widths { handle, impressions, likes }`, computed once in `render` from the replies (widest handle incl. `@`, widest impressions, widest likes, all by digit/char count).
   - `render_line(reply, tz, &Widths)`.
   - Line format: `time  handle  preview  {impressions:>w} impressions  {likes:>w} likes  [link]`. Likes are right-aligned to the widest like count.
   - Header and empty-day output are unchanged.

`main` and `window` are untouched.

### Implementation order (TDD)

1. `api`: `replies` maps `like_count` into `Reply.likes` (update the existing fixtures to include `like_count`).
2. `api`: a reply missing `like_count` maps to `ApiError::Protocol`.
3. `report`: refactor to `Widths`, with no behaviour change. The existing tests stay green.
4. `report`: a line shows `N likes` between impressions and `[link]`. Cover 0 (`0 likes`) and 1 (`1 likes`).
5. `report`: like counts are right-aligned across replies (e.g. `5` and `1234`), extending `aligns_columns_across_replies`.
