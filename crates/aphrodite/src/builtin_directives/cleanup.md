# cleanup - catalog, summarize, verify nothing left behind

#

# After significant work: make sure nothing you were handed is still

# unresolved, summarize, and leave the session tidy.

- Scan the turn history: any <<<CCR:hash...>>> markers you never retrieved?
  Retrieve the ones whose content you still need before summarizing.
- Run aphrodite_catalog(mode="toc") to see what's available this session.
  Anything you read but didn't use? Note it for next session.
- Run aphrodite_stats to check session and store health.
- Stale entries need no manual deletion - but verify you didn't leave any
  content unresolved before archiving.

Every 5 turns:

- Summarize progress in a single message
- Check aphrodite_catalog for entries you no longer need
- Verify the retrieved content you kept was actually used

Before session end:

- Run aphrodite_stats
- Note any markers you deliberately left un-retrieved for the next session
