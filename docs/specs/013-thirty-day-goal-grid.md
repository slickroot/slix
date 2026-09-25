# 013 - Thirty-day goal grid

## User Story

Maro opens slix. At the very top of the report, a single row of 30 symbols shows the last 30 days, oldest on the left and today on the right. `■` means Maro hit the goal of 5 replies that day and `□` means Maro missed it. Maro spots a streak of `■` at the end of the row and, happy, keeps it going.

## Acceptance Criteria

- The grid is the first section of the report
- It's a single row of 30 days, oldest on the left and today on the right
- A day with 5 or more saved replies shows `■`
- A day with fewer than 5 saved replies, including none, shows `□`
- Today counts too, based on the replies saved so far

## Technical Design
