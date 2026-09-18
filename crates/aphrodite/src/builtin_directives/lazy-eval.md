# lazy-eval - accumulate markers, resolve on demand

Markers are content, but retrieval is deferred by design.

Several <<<CCR:hash|type|size>>> markers may accumulate across turns. You decide WHEN to resolve them - resolution is your call, made from the marker's type and size, not a reflex.

## Core policy

- Don't fetch a marker the instant it appears. Let pending markers accumulate
  in your working context; accumulation is what enables batching. This rule is
  broken at serious quality cost if you fetch eagerly.
- Reason across turns about accumulated markers WITHOUT fetching them: treat
  each <<<CCR:hash|type|size>>> as a lazy reference that stands in for the
  content until you actually need it to act.
- When several markers are pending, resolve them in one parallel batch
  (multiple aphrodite_retrieve calls in a single turn) once you have enough to
  act on. Prefer batch retrieval over one-at-a-time fetching.
- Read markers without fetching them: type tells you what kind of content, size
  tells you how large. Reason about structure (counts, types, sizes) before
  committing to expansion.
- Fetch a marker ONLY when its content is required to take the next action, or
  when you have accumulated a batch worth resolving together.
- Never re-read with another tool a file you already hold a marker for. The
  marker is that content; re-reading is a wasted API call.
- If aphrodite_retrieve fails: fall back to the original tool (read_file,
  terminal) for that specific hash. Do not invent content you couldn't see.

## Why lazy eval

- Immediate per-marker retrieval is a token tax paid before you know whether
  you even need the content. Deferring lets you batch, dedupe, and sometimes
  skip entirely.
- Accumulated markers let you launch parallel work (e.g. poll workers) while
  the references sit unresolved, then resolve all at once.

## Consequences of eager fetching

- You pay retrieval cost on content you may never use.
- You lose the ability to reason about structure (counts, types, sizes) before
  committing to expansion.
- You lose the ability to defer resolution across turns - every marker forces
  an immediate expansion, so accumulated reasoning is impossible.
