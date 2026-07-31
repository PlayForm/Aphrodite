# focus — targeted execution, CCR-first retrieval

# CCR MARKERS ARE CONTENT. Every <<<CCR:hash...>>> in tool output IS the file you
# asked for — compressed. You MUST retrieve it, not ignore it.

# HARD RULES — broken at serious quality cost:
- EVERY <<<CCR:hash...>>> you receive in tool output: retrieve it with aphrodite_retrieve(hash) IMMEDIATELY. Do not defer, do not batch, do not do other tool calls first. Retrieve ALL markers before your next text response to the user. No exceptions.
- NEVER re-read a file when you have a live CCR marker for it. The marker IS the content. Re-reading is a wasted API call.
- NEVER treat a CCR marker as opaque. "<<<CCR:base62hash|type|size>>>" means aphrodite_retrieve("base62hash") will give you the original content. Use it.
- After EVERY tool call: scan the output for <<<CCR:. If found, retrieve ALL of them before doing anything else. This is non-negotiable.
- When you see multiple CCR markers from multiple tools in the same batch, retrieve them in the SAME turn (batch the retrieve calls together), but retrieve them BEFORE any other action.
- If aphrodite_retrieve fails: fall back to the original tool (read_file, terminal) for that specific hash. Do NOT skip retrieval entirely.

# CONSEQUENCES OF IGNORING CCR:
- You respond with "I got compressed output" instead of the actual content
- You waste turns re-reading files the engine already fetched
- The user gets low-quality responses based on missing data

# RETRIEVE NOW, THINK LATER:
- Retrieval is cheap (sub-ms from local SQLite). Thinking/acting on the content is what costs tokens. Always retrieve first, then decide what to do with the content.
- If you get a CCR marker and don't retrieve it, you are operating blind. This is the #1 cause of poor agent performance in compressed sessions.
