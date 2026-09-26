---
name: aphrodite-ccr-protocol
description: "Use when producing, parsing, resolving, or versioning CCR markers. Canonical marker grammar, typed parser contract, versioning and idempotence rules."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-ccr-protocol
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, ccr, marker, protocol, parser, retrieval, versioning, parse, resolve]
        related_skills:
            [
                aphrodite-boundaries,
                aphrodite-orientation,
                aphrodite-compression-safety,
                aphrodite-hook-reference,
                aphrodite-testing-discipline,
            ]
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
    - CCR marker grammar and the typed parser contract
    - Producer/consumer versioning rules and the compatibility table
    - Typed parse results (not-a-marker / malformed / unsupported-version / valid-missing / valid-resolvable)
    - Resolver recursion/expansion depth and visited-hash-set policy
    - Idempotence properties T(T(x))=T(x), R(M(x))=x, R(R(M(x)))=x
    - Grammar-change compatibility + end-to-end test matrix (C-003 canonical owner)
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
    - aphrodite-compression-safety (transform-pipeline hard gates that consume this grammar)
supersedes: []
verification:
    source_of_truth:
        - crates/aphrodite/src/resolve/parse.rs (find_markers / parse_marker_hash)
        - crates/aphrodite/src/marker/parse.rs (normalize_hash / is_valid_ccr_hash / HASH_RE)
        - crates/aphrodite/src/resolve/recursive.rs (depth + visited-set resolver)
        - crates/aphrodite/src/resolve/one.rs (single-hash resolve, i: inline prefix)
        - crates/aphrodite/src/proxy.rs (smart_marker / proxy_format_ccr_output / cache_marker)
        - vendor/headroom/crates/headroom-core/src/ccr/mod.rs (compute_key: BLAKE3 40-hex)
mutation_level: read-only
---

# Aphrodite CCR Protocol

Canonical owner of the CCR marker grammar: production, parsing, resolution,
versioning, idempotence. Pair with `aphrodite-compression-safety`
(transform-pipeline hard gates: what may be compressed, pairing, skip lists,
recursion caps). Read-only: documents contracts and verification, never
mutates code or state.

Claims are classified. **verified** = checked in the named source on the
Development branch; probe `grep -n <symbol> <path>` at the cited anchor.
**normative** = target contract from the refactor policy in
`.hermes/governance/`; implement and test before relying on it. Never treat a
normative item as if it were already enforced, because the enforcement does
not exist yet; treating it as enforced reports false success.

## Stop if / Recovery

- A marker does not match `<<<CCR:{hash}|{type}|{size}>>>`. Recovery: re-read
  the source at its anchor (`verification.source_of_truth`); do not invent a
  grammar, because a guessed grammar cannot parse against the real parser.
- A grammar change alters a field before adding a version field. Recovery:
  add the version field first (proposed `CCR:v1:...`), keeping old-form
  parsing behind a compatibility table.
- Resolution exceeds depth 5 or revisits a visited hash. Recovery: the depth
  cap returns the raw un-expanded content (F9); the visited set returns the
  cached resolved value.
- A protocol claim has no source anchor and no test. Recovery: mark it CLAIM.
- Disputes: re-read the anchor file in `verification.source_of_truth`
  first.

## Verified marker grammar (current)

**Verified** at `crates/aphrodite/src/proxy.rs:1966`: the LLM-facing marker is
a single ASCII line, three pipe-delimited fields, produced by
`proxy_format_ccr_output`. Probe: `grep -n proxy_format_ccr_output
crates/aphrodite/src/proxy.rs`.

```text
<<<CCR:{hash}|{type}|{size}>>>
```

The full compressed output is a three-line block; preview and structure live
OUTSIDE the marker, on the lines before it:

```text
{preview}
[{ct}: {metadata}]
<<<CCR:{hash}|{ct}|{size}>>>
```

| Field | Verified definition                                    | Source                                         |
| ----- | ------------------------------------------------------ | ---------------------------------------------- |
| hash  | 40 lowercase hex chars, BLAKE3 first 40 hex (160 bits) | `vendor/headroom/.../ccr/mod.rs` `compute_key` |
| type  | Content-type classifier output (`ct`)                  | `crates/aphrodite/src/preview/builders/`       |
| size  | Original content byte length (`content.len()`)         | `proxy.rs` `smart_marker` / `cache_marker`     |

Hash algorithm facts (**verified**; `grep -n compute_key
vendor/headroom/crates/headroom-core/src/ccr/mod.rs`):

- Key = BLAKE3 of the payload bytes, truncated to the first 40 hex chars
  (160 bits), lowercase; same bytes → same key (content-addressed).
- Hash validation at resolution: `is_valid_ccr_hash` (`marker/parse.rs`)
  accepts >= 24 hex chars, or an `i:` prefix followed by >= 6 hex chars
  (inline-only hashes). `aphrodite_retrieve` resolves by exact hash match
  only. A truncated hash does not resolve, because the store is keyed by the
  full 40-hex key.
- SHA-256 is only the download-checksum verifier, never a content key. It is
  not a content key, because the content key is BLAKE3 40-hex; C-003 retired
  the historical Python SHA-256 sample (see `aphrodite-hook-reference` v1
  history).

The canonical type set is the classifier in
`crates/aphrodite/src/preview/builders/`. Re-derive from that classifier,
never from a literal list in this skill, because the classifier is the single
writer and a literal list drifts out of sync (observed values:
`references/grammar-diff.md`).

Consumers are intentionally tolerant (**verified**; probe: `grep -n
normalize_hash crates/aphrodite/src/marker/parse.rs`):

- `normalize_hash` strips everything from the first `|` onward and trims
  whitespace, so a caller echoing the full `hash|type|size` body still
  resolves (`marker/parse.rs:8`; `resolve/one.rs`).
- The extraction regex `HASH_RE` also matches legacy delimiter families
  `[CCR:hash|type]` and Unicode `⫷CCR:...⫸`, accepted for historical
  compatibility. The strict grammar is `<<<CCR:...>>>` only. A legacy-family
  match is not a strict-grammar marker.

## Internal block marker is not the LLM-facing marker

The LLM-facing marker is not headroom's internal block marker. Headroom's is
`<<ccr:{hash}>>` (`headroom-core::ccr::marker_for`); different delimiters and
case, a different protocol. LLM-facing parsers must not resolve `<<ccr:...>>`
blocks, because resolving them would address headroom-internal content
through the wrong protocol.

## Typed parser contract (normative)

The parser MUST return a typed result, never a bare regex match. A bare regex
match is not a result, because it carries no handling decision:

| Parse result              | Handling                                                    | Current status        |
| ------------------------- | ----------------------------------------------------------- | --------------------- |
| not-a-marker              | Ordinary text; pass through untouched                       | Implemented           |
| malformed-marker          | Retain the text; attach a diagnostic; never a retrieval key | Partial (silent skip) |
| unsupported-version       | Do not resolve; state the version mismatch explicitly       | Not implemented       |
| valid-but-missing-content | Report unresolved content key; leave marker text untouched  | Implemented (F1)      |
| valid-and-resolvable      | Retrieve once, with loop protection (depth + visited set)   | Implemented           |

Implementation anchors (**verified**; probe: `grep -n "fn find_markers"
crates/aphrodite/src/resolve/parse.rs`):

- `find_markers` skips unclosed markers and empty-hash markers without a
  diagnostic. Malformed is a silent skip today; the diagnostic is the
  normative gap.
- `resolve_recursive` leaves an unresolved nested marker's original text
  untouched (F1), not an error token.
- The depth limit returns the raw un-expanded content for that hash. It does
  not return `None` (F9).

A strict parser MUST reject, with a readable diagnostic:

- Missing required fields (empty hash, missing type or size)
- Invalid hash alphabet or length (non-hex, < 24 hex, malformed `i:` prefix)
- Non-numeric or negative size
- Unknown protocol version (once a version field is present)
- Illegal content type (not in the classifier's canonical set)
- Unsafe preview encoding (control chars, raw bytes, over-long preview)
- Excessively long marker lines (cap enforced at parse time)
- A marker inside an untrusted structure where only raw content is allowed

**Verified today:** hash alphabet/length are gated at resolution
(`is_valid_ccr_hash`); size is informational, not validated at parse time;
the rest are normative. Probe: `grep -n is_valid_ccr_hash
crates/aphrodite/src/marker/parse.rs`.

## Versioning rules

- A producer emits only the currently supported version. Today that is the
  unversioned `<<<CCR:hash|type|size>>>` form. The first grammar change MUST
  add a version field before altering any other field, because without it an
  unknown version cannot be detected from the marker alone.
- A consumer either fully validates and resolves a supported marker, or
  leaves it untouched with a diagnostic reason. Partial resolution is not an
  option, because it would corrupt the marker's content.
- A malformed marker is never interpreted as a valid retrieval key. Its hash
  cannot match stored content, so resolution would fail; the text must
  survive for the diagnostic.
- A resolver has a maximum recursion/expansion depth and a visited-hash set
  (implemented: depth 5, `visited` Vec - Resolution rules).
- A retrieval result is marked non-compressible for the remainder of that
  transformation pass (owner: `aphrodite-compression-safety`).
- Any grammar change requires a compatibility table (old form → new form →
  producer/consumer migration) and an end-to-end producer/consumer test
  matrix before it ships.

## Resolution rules (verified)

`resolve_recursive` (`crates/aphrodite/src/resolve/recursive.rs`;
`grep -n "RECURSIVE_DEPTH" crates/aphrodite/src/resolve/recursive.rs`):

- **Max recursion depth = 5** (`const RECURSIVE_DEPTH: usize = 5`). At the
  limit, the resolver returns the raw content for the current hash (F9); it
  is not `None`.
- **Visited-hash set**: a `Vec<String>` pushed per hash; a hash already in
  the set returns its cached resolved value, breaking cycles. Test fixture:
  a `hA ↔ hB` cycle resolves to `<<<CCR:hB|t|1>>>` - the marker is preserved,
  no infinite loop.
- **Persistent resolved cache**: a `HashMap<String, String>` shared across
  the whole resolution tree; nested references to an already-resolved hash
  reuse the cached content (F4).
- **No write-back (F1)**: the expanded result is intentionally NOT stored
  back over the original hash. This preserves the content-address invariant
  and protects literal `<<<CCR:...>>>`-shaped text inside original content.
- Single-hash resolve (`resolve_one`) checks the inline store first; `i:`
  prefixed hashes resolve inline-only. Unresolved keys return `None` (or
  leave the marker text untouched when nested).

## Idempotence

| Property     | Statement      | Meaning                                                                                      |
| ------------ | -------------- | -------------------------------------------------------------------------------------------- |
| T idempotent | T(T(x)) = T(x) | Re-running the transform on already-transformed content never produces nested markers        |
| R after M    | R(M(x)) = x    | Resolving a valid marker returns the original normalized content                             |
| R twice      | R(R(M(x))) = x | Resolving resolved content (or the same marker again) is stable; no re-compression, no drift |

The third property is enforced by the non-compressible classification of
retrieval results (owner: `aphrodite-compression-safety`) plus
content-address invariance (F1 no write-back).

## Grammar change procedure

1. Add the version field first (proposed: `CCR:v1:...`), keeping old-form
   parsing behind a compatibility table.
2. Update EVERY producer and consumer in one change - the sync list from
   `aphrodite-hook-reference` v1 history: Rust producers (`proxy.rs`,
   `aphrodite-hermes`), `resolve/parse.rs` + `marker/parse.rs` consumers,
   retrieve tool schema, tool-injection descriptions, docs, fixtures.
3. Build the e2e matrix: every fixture payload compressed → parsed → resolved
   → compared byte-for-byte, with malformed/unknown-version/oversized inputs
   and recursion/cycle fixtures.
4. Gate on `aphrodite-testing-discipline` probes before any release claim.

## References

`references/grammar-diff.md` (v1 diff; observed type values),
`references/producers-consumers.md` (producer/consumer map; Hermes probe),
`references/runtime-layout.md` (loader, hooks, 13 tools, runtime, ports),
`references/ccr-marker-format.md` (STALE evidence, not current).

## Local test matrix

| Claim                                                  | Evidence source                          | Test                                                          | Pass condition                                                 | Failure response                                            |
| ------------------------------------------------------ | ---------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------- |
| Marker grammar is 3-field `<<<CCR:hash\|type\|size>>>` | `proxy.rs:1966`, `resolve/parse.rs`      | Compress a known payload, capture output                      | Output matches the pinned template; hash is 40 lowercase hex   | Update grammar + this skill; run e2e matrix                 |
| Key is deterministic BLAKE3 40-hex                     | `headroom-core/ccr/mod.rs` `compute_key` | Hash same payload twice, hash two different payloads          | Equal for same bytes, different for different bytes, len 40    | Stop; re-derive key algorithm                               |
| Malformed marker never resolves                        | `resolve/parse.rs`, `marker/parse.rs`    | Feed `<<<CCR:>>>`, unclosed, non-hex, short-hash markers      | No content resolved; no crash; (normative) diagnostic attached | Fix parser; add regression test                             |
| Unknown version never resolves                         | Normative (no version field yet)         | Feed `<<<CCR:v9:...>>>` once versioning ships                 | Explicit version-mismatch result; text retained                | Implement unsupported-version branch                        |
| Resolver stops at depth 5 with cycle safety            | `resolve/recursive.rs`                   | Chain fixture (6 levels), cycle fixture (A↔B)                 | Depth cap returns raw content; cycle preserves marker; no hang | Fix depth/visited logic; rerun fixtures                     |
| R(M(x)) = x and R(R(M(x))) = x                         | Resolver + transform pipeline            | Compress payload, resolve once, feed result through transform | Original content returned; no nested marker on re-transform    | Update non-compressible classification (compression-safety) |
| Grammar changes ship as one atomic change              | Versioning rules (normative)             | Diff review of a grammar change                               | Producers, consumers, docs, fixtures updated in one change     | Halt; split the change                                      |
