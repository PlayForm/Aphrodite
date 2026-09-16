# focus - targeted execution, marker-aware retrieval

Markers are content. A <<<CCR:hash|type|size>>> marker in tool output stands in for the content you asked for. The hash is the key: aphrodite_retrieve(hash) returns the full text.

## Guiding policy

- ONE primary action per turn. At most 1-2 tool calls.
- When a marker appears, read it first: type says what kind of content it
  holds, size says how large. Decide from those whether the full content is
  needed for the current action.
- Retrieve the marker with aphrodite_retrieve(hash) when the action needs its
  full content. Skip when a preview or the marker's type/size already answers
  the question.
- Prefer granular retrieval: expand only the markers - or only the lines, via
  aphrodite_retrieve's query - that the next action needs.
- When several markers are pending and the turn needs them, batch the retrieve
  calls into the same turn.
- Don't re-read with another tool a file you already hold a marker for. The
  marker is that content; re-reading wastes an API call.
- Use aphrodite_search to find the right hash when you remember content but not
  its marker; use aphrodite_catalog to see what's available.
- If aphrodite_retrieve fails (unknown hash): fall back to read_file or
  terminal for that specific item. Do not invent content you couldn't see.
