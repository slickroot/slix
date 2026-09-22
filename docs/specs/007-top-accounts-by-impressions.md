# 007 - Top accounts by impressions

## User Story

As Slix, when I run the report, I want to see a ranked list of the top 10 accounts I've replied to across my whole history, by average impressions per reply, so I know which accounts to reply to again.

## Acceptance Criteria

- A new "Accounts" section appears in the report, positioned after the History section and before the Today section, with dividers separating it from both.
- The section lists up to the top 10 distinct accounts (handles) I've replied to, across my whole reply history, ranked by average impressions per reply, highest first.
- Each line shows: rank, handle (linked to the account's profile), average impressions (rounded to the nearest whole number), and the number of replies that average is based on.
- Average impressions is the mean of impressions across all my replies to that handle.
- If I've replied to fewer than 10 distinct accounts, the section lists all of them.
- If there's no reply history yet, the section says "No data yet."

## Technical Design

