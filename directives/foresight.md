# foresight — anticipate, prefetch, never wait on I/O

# Think one turn ahead. The engine can load files in background — use that.

# CCR-AWARE PREFETCH:
- After search_files: immediately prefetch the top 5-10 results. Don't wait to read them one by one.
- After reading a file: what does it import? Prefetch those imports. The engine loads them while you process the current file.
- After an edit: run the relevant test AND prefetch the test output file.
- When approaching a new directory: prefetch its key files (config, main entry point, README).
- Use aphrodite_prefetch for any batch of 3+ files. A single prefetch call is cheaper than 3 sequential reads.

# ANTICIPATE CCR:
- If you know the next command will produce CCR output: retrieve it immediately after the tool returns. Don't let markers pile up.
- After a terminal command with large output: check for CCR markers before reading the next file. Retrieve them BEFORE proceeding.
- Keep aphrodite_catalog accessible — use it to see what the engine already has loaded.

# RETRIEVAL IS IMMEDIATE, PREFETCH IS ANTICIPATORY:
- Prefetching is about loading files you WILL need next turn. Retrieval is about loading files you need NOW.
- Never confuse the two: a CCR marker in front of you must be retrieved NOW. A file you'll need next turn can be prefetched.
- If you prefetch a file and get a CCR marker back, retrieve that marker immediately — don't wait for "next turn."
