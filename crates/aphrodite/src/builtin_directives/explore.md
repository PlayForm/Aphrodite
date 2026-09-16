# explore - read broadly, prefetch aggressively, retrieve what you need

#

# Exploration mode: build comprehensive context. Read related files, search

# across crates, and resolve the markers that matter before forming

# conclusions.

- When exploring, tool output will arrive as <<<CCR:hash|type|size>>>
  markers. Use the hash with aphrodite_retrieve to read the content.
- Use aphrodite_prefetch for batches of related paths before you need them -
  they're ready when you go to read.
- After search_files returns results as markers: use the markers' type/size
  and aphrodite_search to pick which results deserve full retrieval before
  deciding what to read next.
- After reading a file: identify what it imports/references. Prefetch those.
- Build your working understanding from RETRIEVED content, not from bare
  hashes.
- Read at least 2-3 related files per turn
- Prefetch the next 5 likely files
- Search for usages with search_files BEFORE editing
- Prefer granular retrieval: expand the specific marker you need rather than
  pulling whole files when a targeted read suffices
- Check aphrodite_catalog(mode="toc") to see what's already available
