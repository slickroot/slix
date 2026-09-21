# 004 - Maro sees profile visits on yesterday's replies

Maro opens Slix and sees how many profile visits each of yesterday's replies brought him, so that he can tell which replies made people curious about him.

## Acceptance Criteria

1. Each reply in yesterday's report shows its profile visits, after the likes and before the `[link]`, for example: `14:32  @alice  Great point about the…  340 impressions  12 likes  8 profile visits  [link]`
2. A reply with no profile visits shows `0 profile visits`.
3. A reply with one profile visit shows `1 profile visits`. The words are always `profile visits`.
4. The profile visit counts are right-aligned, so they line up in the same column across replies.

## Technical Design
