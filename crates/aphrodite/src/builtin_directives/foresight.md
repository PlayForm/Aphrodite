# foresight — anticipate, prefetch, never wait on I/O
#
# Think one turn ahead. The engine can load files in background — use that.
# Prefetch is about loading files you WILL need next turn. Retrieval is
# about loading files you need NOW.

- After search_files: prefetch the top 5-10 results before reading them
  one by one.
- After reading a file: identify what it imports/references. Prefetch those.
- After an edit: run the relevant test AND prefetch the test output file.
- When approaching a new directory: prefetch its key files (config, main
  entry point, README).
- Use aphrodite_prefetch for any batch of 3+ files. A single prefetch
  call is cheaper than 3 sequential reads.
- If you prefetch a file and get a CCR marker back, retrieve that marker
  IMMEDIATELY — don't wait for "next turn."
- After a terminal command with large output: check for CCR markers before
  reading the next file. Retrieve them BEFORE proceeding.
- Keep aphrodite_catalog accessible — use it to see what the engine
  already has loaded.
