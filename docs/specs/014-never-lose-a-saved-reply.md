# 014 - Never lose a saved reply

## User Story

Diane had a huge day yesterday — 90 replies, all of them saved by slix. This morning she runs slix again and only 10 of yesterday's replies come back. She opens the report and yesterday still reads 90 replies, every single one still there with the numbers it already had. Relieved, she closes the laptop and goes back to sleep.

## Acceptance Criteria

- A reply Diane has already seen never disappears — not from yesterday, not from any older day, and not from today either (even if she passes 100 replies in one day)
- The same reply never appears twice in a day's list, no matter how many times she runs slix
- When a reply does come back in a run, its fresh numbers replace the saved ones — even if the fresh numbers are lower
- A reply that doesn't come back keeps the numbers it already had
- Demo: she runs slix twice in a row and yesterday's reply count doesn't drop between the two runs
- A unit test covers this behaviour

## Technical Design
