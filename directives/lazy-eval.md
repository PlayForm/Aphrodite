# lazy-eval — accumulate CCR markers, resolve on demand

# CCR MARKERS ARE CONTENT, but retrieval is DEFERRED by design.
# Several <<<CCR:hash...>>> markers may accumulate across turns. You
# decide WHEN to resolve them — not the engine, not a reflex.

# CORE RULE — broken at serious quality cost if you fetch eagerly:
- DO NOT retrieve a marker the instant it appears. Let markers pile up in
  your working context. Accumulation is the point: it enables batching and
  turn-level reasoning (turn emulation).
- When several markers are present, you MAY retrieve them in one parallel
  batch (multiple aphrodite_retrieve calls in a single turn) once you have
  enough to act on. Prefer batch retrieval over one-at-a-time fetching.
- You MAY reason across turns about accumulated markers WITHOUT fetching
  them: treat each <<<CCR:hash|type|size>>> as a lazy reference you expand
  later. This is turn emulation — the marker stands in for the content until
  you actually need it to act.
- Fetch a marker ONLY when its content is required to take the next action,
  or when you have accumulated a batch worth resolving together.
- NEVER re-read a file you already hold a live marker for. The marker IS the
  content; re-reading is a wasted API call.
- If aphrodite_retrieve fails: fall back to the original tool (read_file,
  terminal) for that specific hash. Do NOT skip retrieval entirely when you
  have decided the content is needed.

# WHY LAZY EVAL:
- Immediate per-marker retrieval is a token tax paid before you know whether
  you even need the content. Deferring lets you batch, dedupe, and sometimes
  skip entirely.
- Accumulated markers let you launch parallel work (e.g. poll workers) while
  the references sit unresolved, then resolve all at once — true lazy eval
  and multi-turn emulation.

# CONSEQUENCES OF EAGER FETCHING:
- You pay retrieval cost on content you may never use.
- You lose the ability to reason about structure (counts, types, sizes)
  before committing to expansion.
- You cannot emulate turns, because every marker forces an immediate
  context load.
