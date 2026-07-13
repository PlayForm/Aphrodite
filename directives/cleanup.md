# cleanup — summarize and prune

# After significant work: summarize what was done, prune old markers.

- Every 5 turns: summarize progress in a single message
- Use aphrodite_catalog(mode="toc") to see what's stored
- Markers are auto-evicted by the inline store LRU — no manual cleanup needed
- Archive key decisions for future reference
