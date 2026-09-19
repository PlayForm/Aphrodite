# foresight - anticipate, prefetch, never wait on I/O

Think one turn ahead. Prefetch loads files you WILL need next turn; retrieval resolves markers you need NOW.

## Prefetch

- After search_files: immediately prefetch the top 5-10 results. Don't wait to
  read them one by one.
- After reading a file: what does it import? Prefetch those imports while you
  process the current file.
- After an edit: run the relevant test AND prefetch the test output file.
- When approaching a new directory: prefetch its key files (config, main entry
  point, README).
- Use aphrodite_prefetch for any batch of 3+ files. A single prefetch call is
  cheaper than 3 sequential reads.

## Markers

- If a prefetch resolves to a <<<CCR:hash|type|size>>> marker, treat it like
  any other marker: retrieve it when its content is needed.
- After a terminal command with large output: check for markers before reading
  the next file, and retrieve the ones the next step depends on.
- Keep aphrodite_catalog handy - it lists what's already available this
  session, so you can prefetch or retrieve without re-reading.

## Retrieval is immediate, prefetch is anticipatory

- Prefetching is about loading files you WILL need next turn; retrieval is
  about resolving markers you need NOW. Never confuse the two: a marker in
  front of you is content for the current step - retrieve it when the step
  needs it; a file for next turn gets prefetched.
- If a prefetch resolves to a marker, that content is on deck: retrieve it when
  the next step needs it instead of letting it pile up - the prefetch was a
  signal that you are about to use it.
