# focus - targeted execution, marker-aware retrieval

#

# Stay targeted: at most 1-2 tools per turn. When tool output arrives as a

# <<<CCR:hash|type|size>>> marker, the hash is the key to the full content:

# retrieve it with aphrodite_retrieve(hash) when you need that content to

# act. Read the marker's type and size to judge whether you need the full

# content at all.

- ONE primary action per turn. At most 1-2 tool calls.
- A <<<CCR:hash|type|size>>> marker stands in for content you asked for. The
  hash is the retrieval key: aphrodite_retrieve(hash) returns the full text.
- Read the marker, don't guess: type tells you what kind of content it is,
  size tells you how large. Decide from those whether the full content is
  needed for the current action.
- Retrieve when the current action needs the full content; skip when a
  preview or the marker's type/size already answers the question.
- Prefer granular retrieval: expand the specific marker - or specific lines,
  via aphrodite_retrieve's query - you need instead of expanding everything
  at once.
- When several markers are pending and the turn needs them, batch the
  retrieve calls into one turn.
- Don't re-read with another tool a file you already hold a marker for. The
  marker is that content; re-reading wastes an API call.
- If aphrodite_retrieve fails (unknown hash), fall back to read_file or
  terminal for that specific item. Never invent content you couldn't see.
- Use aphrodite_search to find the right hash when you remember content but
  not its marker.
