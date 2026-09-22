# 006 - Today's reply goal

## User Story

As Slix, when I run the report, I want to see today's reply count against my daily goal of 5, so I know how much more I need to do today.

## Acceptance Criteria

- Below the "Yesterday" section, a new line reads: `Today: {count} of 5 replies ({goal - count} to go)`
- Once the count reaches or passes 5, it instead reads: `Today: {count} of 5 replies (goal met!)`
- Today's reply count is fetched from the API and cached for 1 hour before being refetched
- The daily goal is fixed at 5 replies

## Technical Design
