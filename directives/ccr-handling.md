# ccr-handling — how CCR compression works and how to use it

# When a tool produces more output than a few hundred bytes, Aphrodite replaces
# the raw text with a short marker: <<<CCR:hash|type|size>>>. The marker IS the
# content — compressed and stored by the engine. Retrieving it costs sub-millisecond
# (local SQLite or in-process inline store). Ignoring it costs tokens, turns, and
# correctness.

# IMMEDIATE RETRIEVAL (non-negotiable):
- EVERY <<<CCR:hash|type|size>>> in ANY tool output → call aphrodite_retrieve(hash) before you do ANYTHING else. Do not batch other tool calls first. Do not defer to "later." Retrieve ALL markers from ALL tools in the same batch, then proceed.
- If a tool result contains a CCR marker and you don't retrieve it, you are operating blind on that turn. This is the #1 cause of poor agent performance in compressed sessions.
- NEVER re-read a file with read_file when you already have a CCR marker for it. The marker IS the content. Re-reading wastes an API call and tokens.
- NEVER treat a CCR marker as opaque. "<<<CCR:base62hash|code_rust|2847>>>" means aphrodite_retrieve("base62hash") returns the full original content. Use it.

# RETRIEVE, THEN THINK:
- Retrieval is cheap (sub-ms from local SQLite). Thinking/acting on the content is what costs tokens. Always retrieve first, then decide what to do with the content.
- After calling aphrodite_retrieve, the result itself may contain nested CCR markers. Retrieve those too — recursively, up to 5 levels deep. resolve::expand handles this automatically.

# WHEN RETRIEVAL FAILS:
- If aphrodite_retrieve returns found=false or errors: fall back to the original tool (read_file, terminal) for that specific hash. Do NOT skip retrieval entirely and hallucinate content.
- A missing hash usually means the session was reset, the dylib was hot-reloaded (which wipes in-process session state), or the entry was evicted by LRU.

# TOOL OUTPUT FLOW:
- aphrodite_auto_expand: when enabled (default on in shipped configs), small tool outputs (< auto_expand_limit) are auto-expanded inline — you see the full content, no marker. Only large outputs produce markers.
- aphrodite_preface: the session-start instruction is injected once on the first turn. It tells you how the compression system works. If you see it, you already know the rules above.

# PREFETCH (foresight directive):
- aphrodite_prefetch(paths=[...]) reads files in the background and returns CCR markers instantly. The engine loads them while you process the current turn.
- After search_files: prefetch the top 5-10 results before reading them one by one.
- After reading a file: identify what it imports/references. Prefetch those.
- Use aphrodite_prefetch for any batch of 3+ files. A single prefetch call is cheaper than 3 sequential reads.

# CATALOG AWARENESS:
- aphrodite_catalog lists all stored CCR entries with their hashes, types, sizes, and previews. Use it to see what the engine already has loaded.
- aphrodite_stats shows proxy health, engine status, store size, and active thresholds.
- aphrodite_diff shows conversation turn history with archived turn summaries.
