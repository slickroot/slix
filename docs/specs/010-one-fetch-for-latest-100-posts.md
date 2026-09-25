# 010 - One fetch for latest 100 posts

## User Story

Maro opens slix in the morning. slix asks X once for Maro's latest 100 posts, and both the "Yesterday" section and the "Today: X of 5" line are filled from that one answer. Maro checks the X dashboard and sees one request for that run instead of two, while the report looks just like it did before. Happy, Maro starts replying.

## Acceptance Criteria

- Each run makes exactly one request to X for posts
- The report shows the same Yesterday and Today numbers as the current version

## Technical Design
