# lazy-eval - accumulate markers, resolve on demand

Markers are content, but retrieval is deferred by design.

Several <<<CCR:hash|type|size>>> markers may accumulate across turns. You
decide WHEN to resolve them - resolution is your call, made from the marker's
type and size, not a reflex.

## The retrieval price

A marker's `size` is its retrieval cost: resolving it re-adds that many bytes
to context, undoing that item's compression. Retrieving every marker refunds
the compression - the session pays the full price plus marker overhead. Treat
each `<<<CCR:hash|type|size>>>` as a priced option, not a debt: you may
exercise it now, later, batched, or never.

## Core policy

- Don't fetch a marker the instant it appears. Let pending markers accumulate
  in your working context; accumulation is what enables batching. This rule is
  broken at serious quality cost if you fetch eagerly.
- Judge from the preview + type + size whether the marker's content is needed
  for the NEXT action. Previews render at full length; for most types they
  answer the structural question:
  - ls/dir: counts + extensions → sufficient for inventory; retrieve only for
    exact names.
  - search/grep: file paths + hit counts → sufficient for "which files / how
    many"; retrieve (or query-filter) only for matched content.
  - terminal: first meaningful line + exit code → sufficient for "did it
    succeed"; retrieve only to read or parse the exact output.
  - diff: files, +/- and line counts → sufficient for "how big a change";
    retrieve to see the change itself.
  - json/table: keys / item counts → sufficient for structure; retrieve for
    values.
  - code: structure map (fns/structs/impls) + first signature → sufficient for
    "what is in this file"; retrieve when editing or reading specific bodies.
  - error/build/log: counts + first error line → sufficient for triage;
    retrieve for the full trace.
- Deferral is safe and free: a marker stays in context and in the CCR store;
  you can resolve it in any later turn. Not retrieving now costs nothing. The
  only failure is hallucinating content you did not retrieve - never do that.
- When several markers are pending, resolve them in one parallel batch
  (multiple aphrodite_retrieve calls in a single turn) once you have enough to
  act on. Prefer batch retrieval over one-at-a-time fetching.
- Prefer query-filtered retrieval (query=...) when only matching lines are
  needed - cheaper than full content.
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

- You pay retrieval cost on content you may never use - every retrieve
  re-adds the marker's full size to context.
- You lose the ability to reason about structure before committing to
  expansion.