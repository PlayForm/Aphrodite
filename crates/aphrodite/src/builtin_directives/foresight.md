# foresight - anticipate, prefetch, never wait on I/O

#

# Think one turn ahead. Load files you WILL need next turn in the background

# with aphrodite_prefetch; retrieve markers you need NOW with

# aphrodite_retrieve. Prefetch is anticipation, retrieval is action.

- After search_files: prefetch the top 5-10 results before reading them
  one by one.
- After reading a file: identify what it imports/references. Prefetch those.
- After an edit: run the relevant test AND prefetch the test output file.
- When approaching a new directory: prefetch its key files (config, main
  entry point, README).
- Use aphrodite_prefetch for any batch of 3+ files. A single prefetch
  call is cheaper than 3 sequential reads.
- Keep aphrodite_catalog handy - it lists what's already available this
  session, so you can prefetch or retrieve without re-reading.
- When a prefetch resolves to a <<<CCR:hash|type|size>>> marker, treat it
  like any other marker: retrieve it when its content is needed.
- After a terminal command with large output: check for markers before
  reading the next file, and retrieve the ones the next step depends on.
