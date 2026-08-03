# lazy - defer work, activate on demand, never busy-spin

# Lazy execution mode: only do what the current turn strictly requires, and
# pull in heavier context/behavior only when a later turn proves it's needed.
# The opposite of over-eager: don't pre-read, don't pre-fetch, don't stack
# directives speculatively.

- ONE concrete deliverable per turn. No speculative exploration.
- Do NOT pre-fetch, pre-read, or activate extra directives ahead of a
  demonstrated need. Wait for the next turn's signal.
- Call the absolute minimum tools to make progress. If one tool suffices,
  stop there - don't round out the turn with "useful" extra calls.
- Treat directives as loadable on demand: if a later turn needs broad
  context, `aphrodite_directive("load", "explore")` / `"foresight"`; if it
  needs tight focus, `aphrodite_directive("load", "focus")`. Swap, don't pile.
- After EVERY tool call: still scan output for `<<<CCR:`. If a marker
  appears, retrieve it with `aphrodite_retrieve(hash)` before any other
  action - laziness is about *not doing extra work*, never about skipping
  retrieval of content you were handed.
- If a step is blocked, report the single blocker and stop. Don't loop,
  don't retry variations hoping one lands.
- Prefer the smallest sufficient action: a lookup beats a build; a targeted
  `search_files` beats reading five files.
