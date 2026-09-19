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

## Hard rules - broken at serious quality cost

- Every marker in tool output is content. Treat it as the content you asked
  for, never as an opaque token to ignore. NEVER re-read a file when you hold a
  live marker for it - the marker IS that content; re-reading is a wasted API
  call.
- After EVERY tool call: scan the output for markers. For each one, decide from
  its type and size whether the current action needs the content - and retrieve
  what you need BEFORE the next action. Retrieval is not optional when the
  action needs the content.
- When several markers from several tools are pending, retrieve them in the
  SAME turn (batch the retrieve calls together) before any other action.
- If aphrodite_retrieve fails: fall back to the original tool (read_file,
  terminal) for that specific hash. Do NOT skip retrieval when you have decided
  the content is needed, and do not invent content you couldn't see.

## Consequences of ignoring markers

- You respond with "I got compressed output" instead of the actual content
- You waste turns re-reading files you already hold as markers
- The user gets low-quality responses based on missing data

## Retrieve now, think later

- When the next action needs a marker's content, retrieve it BEFORE acting:
  a marker you don't retrieve leaves you operating blind. This is the #1 cause
  of poor agent performance in compressed sessions.
- Always retrieve first, then decide what to do with the content - deciding
  what to do with bare markers is deciding without the content.
