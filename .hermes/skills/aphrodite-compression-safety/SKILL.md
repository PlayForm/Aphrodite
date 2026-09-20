---
name: aphrodite-compression-safety
description: "Use when deciding what may be compressed, how messages are sliced, or how recursion and re-compression are prevented in the CCR pipeline. Hard gates + failure behavior."
version: 1.0.0
platforms: [macos]
tags: [aphrodite, ccr, safety, compression, recursion, pairing, fail-open]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - Hard gates: tool-call/tool-result pairing, retrieval/catalog/status/health non-compressibility, no re-emission of resolved payloads, no duplicate recursive resolution
    - Max marker recursion depth policy and the readable-diagnostic requirement
    - Preview caps (bytes, UTF-8 boundaries) and the distinct preview classes
    - Failure-behavior policy assignment for compression and preview transforms (fail open)
    - The retrieval skip list (canonical owner of the non-compressible tool set)
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
    - aphrodite-ccr-protocol (grammar + resolver the gates protect)
supersedes: []
verification:
    source_of_truth:
        - crates/aphrodite/src/resolve/recursive.rs (depth/cycle behavior)
        - crates/aphrodite/src/proxy.rs (thresholds, smart_marker, cache_marker)
        - crates/aphrodite-hermes/src/lib.rs (transform/tool dispatch paths)
        - crates/aphrodite/src/preview/ (preview cap + char-boundary tests)
        - references/recompression-guard.md + references/ccr-infinite-recursion.md (historical evidence)
mutation_level: read-only
---

# Aphrodite Compression Safety

Canonical owner of the hard gates that keep CCR compression from corrupting,
erasing, or looping over user-visible content. These are **hard gates, not
advice**: every transform path in the pipeline obeys them. The marker grammar
and resolver contracts live in `aphrodite-ccr-protocol`; this skill owns what
the pipeline may do to content around those markers.

Claims below are **verified** (checked in source 2026-09-20, Development) or
**normative** (policy target; implement and test before relying on it).

## Hard gates

1. **Preserve complete tool-call/tool-result pairs during message slicing.**
   When `compress()` splits messages into head/middle/tail, the tail boundary
   can orphan a tool result. Extend the boundary to absorb orphan
   `role == "tool"` messages (verified incident: session-discovery #8; rule
   also stated in `aphrodite-boundaries` context rules). A split message list
   that separates a tool call from its result is a safety violation.
2. **Never compress the result of a retrieval, catalog, status, or health
   operation.** Retrieval and diagnostic outputs are classified
   non-compressible for the rest of the pass. The canonical skip set includes
   the retrieve/catalog/status/search/health-family tools; both token and
   cache modes honor it (historical evidence:
   `references/recompression-guard.md`, `references/ccr-infinite-recursion.md`).
   The skip list applies BEFORE the threshold check - skipped tools are never
   compressed regardless of size.
3. **Never re-emit a CCR marker for a value that is already a resolved
   retrieval payload.** Content that arrived via retrieval must stay raw in
   the current pass (see idempotence R(R(M(x)))=x in `aphrodite-ccr-protocol`).
4. **Never recursively resolve an identical marker more than once per
   request.** The resolver's visited-hash set (verified:
   `resolve/recursive.rs`, `visited: Vec<String>`) breaks cycles; a hash
   already visited returns its cached resolved value instead of re-resolving.
5. **Enforce a maximum marker recursion depth.** Verified: `RECURSIVE_DEPTH =
5`; at the limit the resolver returns the raw un-expanded content for the
   current hash (F9) - a readable outcome, not another marker and not `None`
   for a hash that legitimately exists.
6. **Cap preview generation by bytes and preserve valid UTF-8 boundaries.**
   Verified: `[previews] preview_max_chars` (env `APHRODITE_PREVIEW_MAX_CHARS`
    > TOML > default 120; absent key = unlimited); tests assert multibyte
    > truncation on char boundaries (`test_preview_cap_multibyte_truncates_on_char_boundary`).
    > A preview cut mid-codepoint is a bug.
7. **Treat binary, JSON, code, and multiline text as distinct preview
   classes.** The core builder (`crate::preview::build_preview`) is the single
   shared producer for both proxy and hook paths - one class per content
   shape, never a flat truncation for everything (verified: `proxy.rs:1848-1851`,
   the parallel preview builder was removed).
8. **Always preserve enough preview information for the model to decide
   whether it should retrieve the payload.** A misleading preview (false line
   count, false byte size, collapsed shape) is a functional failure, not a
   cosmetic issue (AGENTS.md standing rule; rewrite.md §Preview testing).

## Failure-behavior policy

Every hook and transform selects exactly one policy; never leave it implicit:

| Policy              | Use when                                               | Behavior                                             |
| ------------------- | ------------------------------------------------------ | ---------------------------------------------------- |
| **Fail open**       | Observability, optional optimization, preview          | Log structured error; return original content        |
| **Fail closed**     | Security redaction, incompatible validation            | Stop operation with readable diagnostic              |
| **Degrade**         | Upstream proxy unavailable                             | Retain local/raw behavior and expose degraded status |
| **Retry boundedly** | Transient local socket/process start                   | Limited retries with backoff; then degrade           |
| **Escalate**        | Data loss, invalid marker grammar, release side effect | Halt workflow; require human decision                |

**Compression and preview transforms ordinarily FAIL OPEN**: a compression
bug must never erase terminal or tool output. Concretely: a transform
handler that raises, times out, or fails validation returns the original
content, logs the error, and never replaces output with an empty string.
(The historical `stdout` vs `output` bug - wrong parameter names defaulting
to `""` and replacing ALL terminal output with empty - is exactly the class
of failure fail-open prevents; canonical fix in `aphrodite-hook-contracts`.)
Artifact compatibility and registry publishing fail closed; a missing
optional release asset degrades with a warning (owner: release skills).

## Verified pipeline facts

- Thresholds are live-read configuration, never literals: tool output is
  compressed above the token/cache thresholds (`>1KB` token, `>8KB` cache in
  historical docs; current values come from `threshold_for(ct)` +
  `state.compress_threshold()` + `APHRODITE_INLINE_THRESHOLD` in
  `crates/aphrodite/src/proxy.rs`). Read the active config before testing
  any threshold boundary.
- Proxy stores: token mode = SQLite (`:9798`, `ccr.db`), cache mode =
  in-memory (`:9797`); both share CCR stores so content compressed via either
  resolves via either. The inline store is session-scoped Rust state
  (`AphroditeState.inline_store`); a dylib hot-reload wipes it, making every
  existing transcript marker an unresolvable dead reference (plugin warns on
  reload; do not treat reload as a cosmetic operation).
- Unresolved nested markers are left as their original text (F1) - the
  pipeline never substitutes a placeholder token into the store.

## Reference evidence

- `references/recompression-guard.md` - retrieval-tool skip list history and
  the guard pattern (historical; the plugin is now a pure loader and the
  skip logic lives in the dylib).
- `references/ccr-infinite-recursion.md` - the 2026-06-15 infinite-recursion
  incident (historical tool names `headroom_retrieve`/`headroom_stats`; the
  current tool family is `aphrodite_*`).

## Local test matrix

| Claim                                              | Evidence source                    | Test                                                               | Pass condition                                                | Failure response                            |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------- | ------------------------------------------- |
| Tool-call/tool-result pairs survive slicing        | `compress()` boundary logic        | Feed a message list ending in tool results; slice at tail boundary | No orphaned tool result; pair intact                          | Fix boundary extension; add regression test |
| Retrieval/catalog/status/health results stay raw   | Skip set in transform path         | Run `aphrodite_retrieve`/`aphrodite_stats` with large output       | Output never becomes a new marker                             | Update skip set; rerun nested-marker test   |
| Resolved payload is never re-emitted as a marker   | Transform pipeline + idempotence   | Feed a retrieval response through the transform                    | Payload stays raw; no nested marker                           | Fix non-compressible classification         |
| Identical marker resolves once per request         | `resolve/recursive.rs` visited set | Cycle fixture A↔B; duplicate marker in one payload                 | No re-resolution; stable output; no hang                      | Fix visited-set logic                       |
| Recursion stops at depth 5 with readable outcome   | `RECURSIVE_DEPTH = 5` (F9)         | Chain fixture longer than 5 levels                                 | Raw content returned at cap; no marker, no crash              | Fix depth handling                          |
| Preview cap preserves UTF-8 boundaries             | `preview/state.rs` + tests         | Multibyte payload truncated at cap                                 | No broken codepoint in preview                                | Fix truncation to char boundary             |
| Distinct preview classes produce truthful previews | `preview/builders/`                | Binary / JSON / code / multiline fixtures                          | Shape, line count, byte size stated correctly                 | Fix the class builder                       |
| Compression failure never erases output            | Fail-open policy                   | Inject a raising transform handler                                 | Original content returned; error logged; no empty replacement | Enforce fail-open default in all transforms |
