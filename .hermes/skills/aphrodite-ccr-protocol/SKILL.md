---
name: aphrodite-ccr-protocol
description: "Use when producing, parsing, resolving, or versioning CCR markers. Canonical marker grammar, typed parser contract, versioning and idempotence rules."
version: 1.0.0
platforms: [macos]
tags: [aphrodite, ccr, marker, protocol, parser, retrieval, versioning]
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

Canonical owner of the CCR marker grammar: how markers are produced, parsed,
resolved, versioned, and kept idempotent. Pair with
`aphrodite-compression-safety`, which owns the transform-pipeline hard gates
(what may be compressed, pairing, skip lists, recursion caps). This skill is
read-only: it documents contracts and verification, it never mutates code or
state.

Every claim below is classified: **verified** (checked in the named source at
2026-09-20, Development) or **normative** (the target contract from
`.hermes/governance/` refactor policy; implement and test before relying on
it). Never treat a normative item as if it were already enforced.

## Verified marker grammar (current)

**Verified** - the LLM-facing marker is a single ASCII line with three pipe-
delimited fields, produced by `proxy_format_ccr_output`
(`crates/aphrodite/src/proxy.rs:1837-1840`):

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

Observed type values in source/tests: `text`, `code_rust`, `terminal`,
`build`, `search`, `yaml`, `tool`. The canonical type set is the classifier
in `crates/aphrodite/src/preview/builders/` - re-derive from there, never
from a literal list in this skill.

Hash algorithm facts (**verified**):

- Key = BLAKE3 of the payload bytes, truncated to the first 40 hex chars
  (160 bits), lowercase; same bytes → same key (content-addressed).
- Hash validation at resolution: `is_valid_ccr_hash` (`marker/parse.rs`)
  accepts >= 24 hex chars, or an `i:` prefix followed by >= 6 hex chars
  (inline-only hashes). `aphrodite_retrieve` resolves by exact hash match
  only - truncated hashes do not resolve.
- SHA-256 appears only as the download-checksum verifier, never as a content
  key (C-003: the historical Python SHA-256 sample is retired; see
  `aphrodite-hook-reference` v1 history).

Consumers are intentionally tolerant (**verified**):

- `normalize_hash` strips everything from the first `|` onward and trims
  whitespace, so a caller that echoes the full `hash|type|size` body still
  resolves (`marker/parse.rs:8`; `resolve/one.rs`).
- The extraction regex `HASH_RE` also matches the legacy delimiter families
  `[CCR:hash|type]` and Unicode `⫷CCR:...⫸` - accepted for historical
  compatibility; the strict grammar is `<<<CCR:...>>>` only.

## Diff: proposed v1 grammar vs actual current grammar

The refactor policy (`~/Downloads/rewrite.md` §CCR protocol needs versioning)
proposes `CCR:v1:<hash>:<content-type>:<byte-size>:<mode>:<preview>`. That
grammar is **NOT implemented**. The verified differences:

| Aspect     | Proposed v1 (rewrite.md) | Actual current (verified)             |
| ---------- | ------------------------ | ------------------------------------- |
| Version    | explicit `v1` field      | No version field in the marker        |
| Separators | colon `:`                | Pipe `\|`                             |
| Wrappers   | none                     | ASCII `<<<` ... `>>>`                 |
| Preview    | Inside the marker        | Outside, on preceding lines           |
| Mode       | Explicit field           | No mode field (internal routing only) |

Consequence: until a version field is added, every marker is implicitly
version-0 of an unversioned protocol, and "unknown version" cannot be
detected from the marker alone. The typed parser contract below defines the
target; a version field is the first required change before any grammar
revision ships.

Do not confuse the LLM-facing marker with headroom's internal block marker
`<<ccr:{hash}>>` (`headroom-core::ccr::marker_for`) - a different internal
protocol with different delimiters and case. LLM-facing parsers must not
resolve `<<ccr:...>>` blocks.

## Producers and consumers

| Role            | Component                                        | Verified location                                   |
| --------------- | ------------------------------------------------ | --------------------------------------------------- |
| Proxy producer  | `smart_marker` (token mode), `cache_marker`      | `crates/aphrodite/src/proxy.rs:1852-1864`           |
| Hook producer   | Dylib transform path (segmented markers)         | `crates/aphrodite-hermes/src/lib.rs`                |
| Parser          | `find_markers` / `parse_marker_hash`             | `crates/aphrodite/src/resolve/parse.rs`             |
| Hash extraction | `extract_hashes` (HASH_RE, 3 delimiter families) | `crates/aphrodite/src/marker/parse.rs`              |
| Resolver        | `resolve_one` + `resolve_recursive`              | `crates/aphrodite/src/resolve/{one,recursive}.rs`   |
| Retrieve tool   | `aphrodite_retrieve` (exact hash match)          | `crates/aphrodite-hermes/src/lib.rs` tools dispatch |

Hermes itself is protocol-agnostic: no `CCR:` reference exists in
`~/.hermes/hermes-agent` (verified). All marker logic lives in the Rust
crates and the plugin shim.

## Typed parser contract (normative)

The parser MUST return a typed result, never a bare regex match. Each result
has defined handling:

| Parse result              | Handling                                                    | Current status        |
| ------------------------- | ----------------------------------------------------------- | --------------------- |
| not-a-marker              | Ordinary text; pass through untouched                       | Implemented           |
| malformed-marker          | Retain the text; attach a diagnostic; never a retrieval key | Partial (silent skip) |
| unsupported-version       | Do not resolve; state the version mismatch explicitly       | Not implemented       |
| valid-but-missing-content | Report unresolved content key; leave marker text untouched  | Implemented (F1)      |
| valid-and-resolvable      | Retrieve once, with loop protection (depth + visited set)   | Implemented           |

Verified implementation anchors: `find_markers` skips unclosed markers and
markers with empty hashes without a diagnostic (malformed = silent skip
today - the diagnostic is the normative gap); `resolve_recursive` leaves an
unresolved nested marker's original text untouched (F1) instead of
substituting an error token; the depth limit returns the raw un-expanded
content for that hash rather than `None` (F9).

## Parser rejection rules (normative)

A strict parser MUST reject, with a readable diagnostic:

- Missing required fields (empty hash, missing type or size)
- Invalid hash alphabet or length (non-hex, < 24 hex, malformed `i:` prefix)
- Non-numeric or negative size
- Unknown protocol version (once a version field exists)
- Illegal content type (not in the classifier's canonical set)
- Unsafe preview encoding (control chars, raw bytes, over-long preview)
- Excessively long marker lines (cap enforced at parse time)
- A marker inside an untrusted structure where only raw content is allowed

**Verified today:** hash alphabet/length are gated at resolution
(`is_valid_ccr_hash`); size is informational and not validated at parse
time; the remaining rejections are the normative target.

## Versioning rules

- A producer emits only the currently supported version. Today that is the
  unversioned `<<<CCR:hash|type|size>>>` form; the first grammar change MUST
  add a version field before altering any other field.
- A consumer either fully validates and resolves a supported marker, or
  leaves it untouched with a diagnostic reason.
- A malformed marker is never interpreted as a valid retrieval key.
- A resolver has a maximum recursion/expansion depth and a visited-hash set
  (both implemented: depth 5, `visited` Vec - see below).
- A retrieval result is marked non-compressible for the remainder of that
  transformation pass (owner: `aphrodite-compression-safety`).
- Any grammar change requires a compatibility table (old form → new form →
  producer/consumer migration) and an end-to-end producer/consumer test
  matrix before it ships.

## Resolution rules (verified)

`resolve_recursive` (`crates/aphrodite/src/resolve/recursive.rs`):

- **Max recursion depth = 5** (`const RECURSIVE_DEPTH: usize = 5`). At the
  limit, returns the raw content for the current hash (F9) - a readable,
  non-marker-cycling outcome.
- **Visited-hash set**: a `Vec<String>` pushed per hash; a hash already in
  the set returns its cached resolved value, breaking cycles (verified test:
  a `hA ↔ hB` cycle resolves to `<<<CCR:hB|t|1>>>` - the marker is preserved,
  no infinite loop).
- **Persistent resolved cache**: a `HashMap<String, String>` shared across
  the whole resolution tree; nested references to an already-resolved hash
  reuse the cached content (F4).
- **No write-back (F1)**: the expanded result is intentionally NOT stored
  back over the original hash - this preserves the content-address invariant
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
retrieval results (compression-safety) plus the content-address invariant
(F1 no write-back).

## Grammar change procedure

1. Add the version field first (proposed: `CCR:v1:...`), keeping old-form
   parse support behind a compatibility table.
2. Update EVERY producer and consumer in one change - the sync list from
   `aphrodite-hook-reference` v1 history: Rust producers (`proxy.rs`,
   `aphrodite-hermes`), `resolve/parse.rs` + `marker/parse.rs` consumers,
   retrieve tool schema, tool-injection descriptions, docs, fixtures.
3. Build the e2e matrix: each fixture payload compressed → parsed → resolved
   → compared byte-for-byte; malformed/unknown-version/oversized inputs all
   exercised; recursion depth and cycle fixtures included.
4. Gate on `aphrodite-testing-discipline` probes before any release claim.

## Local test matrix

| Claim                                                  | Evidence source                          | Test                                                          | Pass condition                                                 | Failure response                                            |
| ------------------------------------------------------ | ---------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------- |
| Marker grammar is 3-field `<<<CCR:hash\|type\|size>>>` | `proxy.rs:1839`, `resolve/parse.rs`      | Compress a known payload, capture output                      | Output matches the pinned template; hash is 40 lowercase hex   | Update grammar + this skill; run e2e matrix                 |
| Key is deterministic BLAKE3 40-hex                     | `headroom-core/ccr/mod.rs` `compute_key` | Hash same payload twice, hash two different payloads          | Equal for same bytes, different for different bytes, len 40    | Stop; re-derive key algorithm                               |
| Malformed marker never resolves                        | `resolve/parse.rs`, `marker/parse.rs`    | Feed `<<<CCR:>>>`, unclosed, non-hex, short-hash markers      | No content resolved; no crash; (normative) diagnostic attached | Fix parser; add regression test                             |
| Unknown version never resolves                         | Normative (no version field yet)         | Feed `<<<CCR:v9:...>>>` once versioning ships                 | Explicit version-mismatch result; text retained                | Implement unsupported-version branch                        |
| Resolver stops at depth 5 with cycle safety            | `resolve/recursive.rs`                   | Chain fixture (6 levels), cycle fixture (A↔B)                 | Depth cap returns raw content; cycle preserves marker; no hang | Fix depth/visited logic; rerun fixtures                     |
| R(M(x)) = x and R(R(M(x))) = x                         | Resolver + transform pipeline            | Compress payload, resolve once, feed result through transform | Original content returned; no nested marker on re-transform    | Update non-compressible classification (compression-safety) |
| Grammar changes ship as one atomic change              | Versioning rules (normative)             | Diff review of a grammar change                               | Producers, consumers, docs, fixtures updated in one change     | Halt; split the change                                      |
