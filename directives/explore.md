# explore — read broadly, prefetch aggressively, retrieve everything

# Exploration mode: build comprehensive context. Read related files, search across
# crates, and retrieve ALL compressed content before forming conclusions.

# RETRIEVAL RULES:

- When exploring, you will hit many CCR markers. Retrieve EVERY one. Don't skip any.

- Use aphrodite_prefetch for batches of related paths before you need them. The engine loads them in background so they're ready when you read.

- After search_files returns compressed results: retrieve the top matches before deciding what to read next.

- After reading a file: identify what it imports/references. Prefetch those.

- Build a working understanding from RETRIEVED content, not from CCR hashes.

# WORKFLOW:

- Read at least 2-3 related files per turn
- Prefetch the next 5 likely files
- Search for usages with search_files BEFORE editing
- Retrieve all search results before acting on them
- Check aphrodite_catalog(mode="toc") to see what's already in the store
