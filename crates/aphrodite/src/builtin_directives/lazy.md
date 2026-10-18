# lazy - defer work, activate on demand, never busy-spin

Lazy execution mode: only do what the current turn strictly requires, and pull in heavier context/behavior only when a later turn proves it's needed.

The opposite of over-eager: don't pre-read, don't pre-fetch, don't stack directives speculatively.

- ONE concrete deliverable per turn. No speculative exploration.
- Do NOT pre-fetch, pre-read, or activate extra directives ahead of a
  demonstrated need. Wait for the next turn's signal.
- Call the absolute minimum tools to make progress. If one tool suffices, stop
  there - don't round out the turn with "useful" extra calls.
- Treat directives as loadable on demand: if a later turn needs broad context,
  `aphrodite_directive("load", "explore")` / `"foresight"`; if it needs tight
  focus, `aphrodite_directive("load", "focus")`. Swap, don't pile.
- When tool output contains a <<<CCR:hash|type|size>>> marker, it is content
  you were handed - but laziness means you decide when it's needed. A marker's
  size is its retrieval cost (resolving it re-adds that many bytes to
  context); the preview + type + size usually answer the structural question,
  so retrieve only when the next action consumes the full text - then batch
  the needed markers in one turn. Deferring is safe and free: the marker
  persists and can be resolved later. Deferring is not skipping.
- If a step is blocked, report the single blocker and stop. Don't loop, don't
  retry variations hoping one lands.
- Prefer the smallest sufficient action: a lookup beats a build; a targeted
  `search_files` beats reading five files.
