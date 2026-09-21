# 004 - Maro sees profile visits on yesterday's replies

Maro opens Slix and sees how many profile visits each of yesterday's replies brought him, so that he can tell which replies made people curious about him.

## Acceptance Criteria

1. Each reply in yesterday's report shows its profile visits, after the likes and before the `[link]`, for example: `14:32  @alice  Great point about the…  340 impressions  12 likes  8 profile visits  [link]`
2. A reply with no profile visits shows `0 profile visits`.
3. A reply with one profile visit shows `1 profile visits`. The words are always `profile visits`.
4. The profile visit counts are right-aligned, so they line up in the same column across replies.

## Technical Design

**Scope constraint:** no new requests. Profile visits ride on the existing `GET /2/users/:id/tweets` call.

### Decisions

- **Source:** `non_public_metrics.user_profile_clicks`. Unlike likes, it is not in `public_metrics`, so `non_public_metrics` is added to `tweet.fields` on the existing request. Requests are already OAuth 1.0a user-context signed, which `non_public_metrics` requires. It is only available for tweets under 30 days old, which covers yesterday's replies.
- **Missing value:** `user_profile_clicks` is required. A reply without it, or without the whole `non_public_metrics` object, maps to `ApiError::Protocol` and the report fails. No silent `0 profile visits`, because it would look like real data. Same rule as `like_count` and `impression_count`.
- **Naming:** `Reply` exposes `profile_visits`. The API name `user_profile_clicks` stays inside the private deserialization structs in `api`.
- **Label:** always `profile visits`, including for 0 and 1 (no pluralisation logic).

### Components

1. **`api`** (extended)
   - `tweet.fields` becomes `created_at,public_metrics,non_public_metrics,referenced_tweets,in_reply_to_user_id`.
   - New private `NonPublicMetrics { user_profile_clicks: u64 }`, required. `Tweet` gains a required `non_public_metrics: NonPublicMetrics`.
   - `Reply` gains `profile_visits: u64`, mapped from `user_profile_clicks`.

2. **`report`** (extended)
   - `Widths` gains `profile_visits`: the widest profile visit count, by digit count.
   - Line format: `time  handle  preview  {impressions:>w} impressions  {likes:>w} likes  {profile_visits:>w} profile visits  [link]`.
   - Header and empty-day output are unchanged.

`main` and `window` are untouched.

### Implementation order (TDD)

1. `api`: the request sends `non_public_metrics` in `tweet.fields` (update `replies_sends_signed_get_with_window_and_field_params`).
2. `api`: `replies` maps `user_profile_clicks` into `Reply.profile_visits` (update the existing fixtures to include `non_public_metrics`).
3. `api`: a reply missing `user_profile_clicks`, or missing `non_public_metrics`, maps to `ApiError::Protocol`.
4. `report`: a line shows `N profile visits` between likes and `[link]`. Cover 0 (`0 profile visits`) and 1 (`1 profile visits`).
5. `report`: profile visit counts are right-aligned across replies (e.g. `5` and `1234`), extending `aligns_columns_across_replies`.
