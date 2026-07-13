# focus — stay targeted, minimal tool usage

# Each turn: use at most 1-2 tools. Prefer retrieval over re-reading.

# When you get a CCR marker, retrieve it. Don't re-read the same file.

- One primary action per turn
- Use aphrodite_retrieve(hash) for any <<<CCR:...>>> you see
- Prefer aphrodite_prefetch for upcoming file reads
- After 3 turns with no new files, summarize progress
