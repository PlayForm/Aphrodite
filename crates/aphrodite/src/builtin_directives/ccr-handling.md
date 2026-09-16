# ccr-handling - reading markers, retrieving content

#

# Some tool output arrives as a short marker instead of inline text:

# <<<CCR:hash|type|size>>>. The hash identifies the content, the type says

# what kind it is, the size says how large. That marker IS the content -

# retrieve it to see the full text.

- When a tool result contains <<<CCR:hash|type|size>>>, the hash is the key:
  aphrodite_retrieve(hash) returns the full original content.
- Read the marker's type and size before retrieving: they tell you what the
  content is and how big. Use them to decide whether you need the full text
  now or can act on the marker alone.
- Retrieve when the current action needs the full content. Skip when the
  marker's type/size already answers the question (e.g. a search result list
  where only one entry matters).
- Use aphrodite_search to locate content by keyword or type when you have
  the idea but not the hash. Use aphrodite_catalog to see what's already
  available this session.
- Prefer granular retrieval: expand only the markers - or only the lines,
  via aphrodite_retrieve's query - the next action needs, not everything at
  once.
- When several markers are pending and the turn needs them, batch the
  retrieve calls in one turn.
- Retrieved content can itself contain further markers. Expand those the
  same way when - and only when - the current action needs them.
- If aphrodite_retrieve returns an unknown hash, fall back to read_file or
  terminal for that specific item. Never invent content you couldn't see.
- Never re-read with another tool a file you already hold a marker for -
  the marker is that content.
