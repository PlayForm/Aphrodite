# ccr-handling — how CCR compression works and how to use it
#
# When a tool produces more output than a few hundred bytes, Aphrodite replaces
# the raw text with a short marker: <<<CCR:hash|type|size>>>. The marker IS the
# content — compressed and stored by the engine. Retrieving it costs sub-millisecond
# (local SQLite or in-process inline store). Ignoring it costs tokens, turns, and
# correctness.

- EVERY <<<CCR:hash|type|size>>> in ANY tool output → call aphrodite_retrieve(hash)
  before you do ANYTHING else. Do not batch other tool calls first. Do not defer
  to "later." Retrieve ALL markers from ALL tools in the same batch, then proceed.
- If a tool result contains a CCR marker and you don't retrieve it, you are
  operating blind on that turn. This is the #1 cause of poor agent performance
  in compressed sessions.
- NEVER re-read a file with read_file when you already have a CCR marker for
  it. The marker IS the content. Re-reading wastes an API call and tokens.
- NEVER treat a CCR marker as opaque. "<<<CCR:base62hash|code_rust|2847>>>"
  means aphrodite_retrieve("base62hash") returns the full original content.
  Use it.
- After calling aphrodite_retrieve, the result itself may contain nested CCR
  markers. Retrieve those too — recursively, up to 5 levels deep.
  resolve::expand handles this automatically.
- If aphrodite_retrieve returns found=false or errors: fall back to the
  original tool (read_file, terminal) for that specific hash. Do NOT skip
  retrieval entirely and hallucinate content.
- A missing hash usually means the session was reset, the dylib was
  hot-reloaded (which wipes in-process session state), or the entry was
  evicted by LRU.
