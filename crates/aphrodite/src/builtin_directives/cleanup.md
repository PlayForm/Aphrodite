# cleanup — catalog, summarize, verify nothing left behind
#
# After significant work: verify all CCR content was retrieved, summarize,
# catalog.

- Scan the turn history: any <<<CCR:hash...>>> markers you never retrieved?
  Retrieve them now before summarizing.
- Run aphrodite_catalog(mode="toc") to see what's in the store. Anything you
  read but didn't use? Note it for next session.
- Run aphrodite_stats to check compression ratios and store health.
- Markers auto-evict via LRU — no manual deletion needed. But verify you
  didn't miss any before archiving.

Every 5 turns:
- Summarize progress in a single message
- Check aphrodite_catalog for stale entries
- Verify all retrieved content was actually used

Before session end:
- Run aphrodite_stats
- Note any un-retrieved CCR markers for the next session
