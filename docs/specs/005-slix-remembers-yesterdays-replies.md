# 005 - Slix remembers yesterday's replies

Maro opens Slix several times in a day and yesterday's replies are fetched from X only once, so that the report is instant and a history of his replies builds up over time.

## Acceptance Criteria

1. The first time Maro opens Slix on a given day, it fetches yesterday's replies from X and saves them with everything the report shows.
2. Every later run that same day shows the saved replies and does not contact X at all. The numbers are those from the first run of the day.
3. If Maro made no replies yesterday, that is saved too. Later runs that day say `No replies yesterday.` without contacting X.
4. If X fails on the first run of the day, nothing is saved, and the next run tries X again.
5. Saved days are kept. The history grows day by day and nothing is thrown away.

## Technical Design
