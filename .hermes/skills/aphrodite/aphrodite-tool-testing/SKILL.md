---
name: aphrodite-tool-testing
description: "Use when teaching or verifying aphrodite CCR tool usage: the 13 CCR tools, marker handling, the retrieve-first rule, read-only vs state-changing classification, and the tool test cases."
version: 2.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-tool-testing
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, ccr, compression, retrieve, testing, tools]
        related_skills:
            [
                aphrodite-boundaries,
                aphrodite-orientation,
                aphrodite-compression-safety,
                aphrodite-auto-expand-testing,
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
    - 13-tool CCR inventory and per-tool behavior matrix
    - Retrieve-first rule (marker -> aphrodite_retrieve before any other action)
    - Read-only vs state-changing classification of CCR tools
    - CCR tool test cases (adapted from the hook test cases)
    - CCR tool usage checklist
depends_on:
    - aphrodite-boundaries (context boundaries: never compress retrieval/diagnostic responses)
    - aphrodite-orientation (preflight gate before verifying a session)
supersedes: []
verification:
    source_of_truth:
        - plugins/aphrodite/plugin.yaml (tool registration list)
        - crates/aphrodite/src/proxy.rs (tool relay dispatch, retrieve hash validation)
        - plugins/aphrodite/README.md (tool behavior descriptions)
mutation_level: read-only
---

# Aphrodite Tool Testing & CCR Retrieval

## When to Use

- You see `<<<CCR:hash|type|size>>>` markers in tool output and must handle
  them correctly instead of losing the content behind them.
- Verifying the CCR engine is healthy, or teaching agents the 13-tool API.

## The 13 CCR Tools (Quick Reference)

**Confidence:** source-derived - `plugins/aphrodite/plugin.yaml` (lines 15-27)
is the registration source of truth; `crates/aphrodite/src/proxy.rs` is the
tool-relay implementation.
**Verify:** diff the table below against `plugin.yaml`'s `tools` list and the
live session tool catalog before teaching it.
**If different:** update this skill and report the drift to the manifest owner.

| Tool                        | Purpose                                              | Key Parameters                                                              | Class          |
| --------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------- | -------------- |
| `aphrodite_stats`           | Check health, version, thresholds, proxy status      | none                                                                        | read-only      |
| `aphrodite_test`            | Smoke test: compress -> retrieve -> search roundtrip | `mode` (quick=1 sample, full=3 samples)                                     | state-changing |
| `aphrodite_compress`        | Compress content into CCR                            | `content` (required), `type` (code/log/diff/error/json/build_output/text)   | state-changing |
| `aphrodite_retrieve`        | **Retrieve original content from CCR**               | `hash` (required), `query` (optional filter), `path` (optional file bypass) | read-only      |
| `aphrodite_search`          | Search CCR entries                                   | `query` (required), `type` (optional filter)                                | read-only      |
| `aphrodite_catalog`         | List all CCR entries                                 | `mode` ("toc" for compact, default full)                                    | read-only      |
| `aphrodite_diff`            | Show conversation turn history                       | none                                                                        | read-only      |
| `aphrodite_rebuild`         | Report binary version + proxy health                 | none                                                                        | read-only      |
| `aphrodite_reclassify`      | Retroactively classify/metadata-enrich entries       | `hash` (optional, omit for all)                                             | state-changing |
| `aphrodite_prefetch`        | Read files in background -> compress to CCR          | `paths` (array of file paths)                                               | state-changing |
| `aphrodite_prefetch_status` | Live prefetch schedule                               | none                                                                        | read-only      |
| `aphrodite_files`           | List all file paths referenced in session            | none                                                                        | read-only      |
| `aphrodite_directive`       | Manage behavioral directives                         | `action`, `name`                                                            | state-changing |

Note on `aphrodite_debug`: the live session tool catalog also exposes
`aphrodite_debug` (toggle per-session debug output on/off, Rust-side only) that
is NOT registered in `plugin.yaml`.
**Confidence:** runtime-derived - verify against the live tool catalog before
relying on it; do not treat it as part of the registered 13.

## Retrieve-first rule

**A CCR marker in tool output is content, not a placeholder.** When a marker
appears, resolve it with `aphrodite_retrieve(hash)` before any other action
that consumes the content it stands for. This is the standing rule in every
session; auto-expand is inert configuration and never resolves markers for you
(see `aphrodite-auto-expand-testing`).

- Never re-read the source file behind a marker with `read_file` - the marker
  IS the content; re-reading just yields another marker.
- Resolve EVERY marker the next action needs, in the same turn. Local tools
  take one call per invocation (a `tool_call` with multiple local entries is
  rejected); issue the retrieves one at a time and complete them all before
  acting. Connector tools may be batched.
- Prefer a `query` filter over full content when only matching lines are
  needed - retrieval is cheap, filtering is cheaper.
- Nested markers are expanded recursively by the resolver.
- Never compress a retrieval response or an Aphrodite diagnostic response
  (context boundary, canonical owner: `aphrodite-compression-safety`);
  resolving a marker is non-destructive and never re-marks the payload.

Why agents fail:

1. They treat the marker as opaque and lose the content it stands for.
2. They re-read the source file with `read_file` instead of retrieving.
3. They retrieve one marker at a time across turns instead of resolving all
   needed markers in the same turn.
4. They rely on auto-expand or "the preview was fine" instead of the canonical
   retrieval path.

## Using `aphrodite_retrieve`

1. **Full content by hash**: `aphrodite_retrieve(hash="abc123...")` ->
   `{found: true, source: "ccr", hash: "...", content: "..."}`
2. **Query-filtered**: `aphrodite_retrieve(hash="abc123...", query="timeout")`
   returns only lines matching the query.
3. **Direct file read (fallback)**: `aphrodite_retrieve(path="README.md")` ->
   `{found: true, source: "path", ...}`. Path-based reads enforce workspace
   containment - paths outside the workspace return `found: false`.
4. **When retrieval fails**: if `aphrodite_retrieve(hash=...)` returns
   `found: false`, fall back to `read_file` / `terminal` on the original path.

## Per-tool behavior notes

- `aphrodite_compress` - `content` is required; the `type` hint wins over
  auto-detection. Returns `hash` + `marker`. The returned marker IS the proof
  of compression; retrieve only when the action needs the content, never to
  "verify storage".
- `aphrodite_retrieve` - `hash` is required for CCR resolution; requests with
  only `query` and no `hash` are rejected with a readable error (validation in
  `crates/aphrodite/src/proxy.rs`).
- `aphrodite_test` - `mode="quick"` runs 1 sample; `mode="full"` (any
  non-quick value) runs the 3-check round-trip set (source_code/build/
  json_array). There is no `matrix`/`pipeline` mode. Expect `status="ok"`.
- `aphrodite_prefetch` - reads + compresses files in the background; markers
  are returned inline. `aphrodite_prefetch_status` shows loading/ready/errors.
  Use prefetch for batches of 3+ files.
- `aphrodite_reclassify` - retroactive metadata enrichment; omit `hash` to
  process all entries.
- `aphrodite_directive` - `action` in list/swap/add/remove/reset. The active
  directives are `focus` (targeted execution, preview-aware retrieval) and
  `foresight` (anticipate I/O: after `search_files`, prefetch the top 5-10
  results).
- `aphrodite_rebuild` - reports binary version + proxy health and a rebuild
  hint; it does NOT rebuild or restart anything.
- `aphrodite_debug` - toggles per-session debug output, Rust-side only; not
  registered in `plugin.yaml` (runtime-derived; see inventory note).

## Read-only vs state-changing

Read-only tools (`stats`, `search`, `catalog`, `diff`, `files`,
`prefetch_status`, `rebuild`, `retrieve`) never mutate session or store state -
running them twice yields identical results and no new CCR entries.
State-changing tools (`compress`, `test`, `reclassify`, `prefetch`,
`directive`, `debug`) write entries, mutate metadata, or change session
behavior - treat each call as deliberate. A verification flow that mutates is
not a read-only verification.

## Never compress retrieval/diagnostic responses

Retrieval, catalog, status, and health results are never compressible
(context boundary; canonical owner: `aphrodite-compression-safety`, per the
skill manifest). Re-emitting a CCR marker for a value that is already a
resolved retrieval payload is forbidden - a retrieval response stays raw
through the transform pass. If a retrieval response ever comes back as a
marker, that is a bug in the skip classifier, not a reason to resolve again.

## Tool test cases (adapted from the hook test cases)

Each active tool needs at least these cases:

| Case                         | CCR-tool application                                           | Pass condition                                                                                        |
| ---------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Valid nominal invocation     | `aphrodite_test(mode="quick")`; compress -> retrieve roundtrip | `status="ok"`; resolved bytes equal source normalized content                                         |
| Unknown extra keyword        | `aphrodite_retrieve(hash=..., bogus=1)`                        | Readable error or schema rejection; no crash, no marker loss                                          |
| Missing optional keyword     | `aphrodite_retrieve(query="x")` with no hash/path              | `{found: false, error}` - proxy rejects hash-less retrieve                                            |
| Empty text input             | `aphrodite_compress(content="")`                               | Defined result (valid empty entry or readable error); never a crash                                   |
| Large payload                | content above the live threshold                               | One valid marker; retrieve returns original bytes; preview truthful                                   |
| Error status / non-zero exit | retrieve unknown hash; proxy error                             | `{found: false, error}` surfaced, never swallowed                                                     |
| Handler exception            | dispatch error inside the tool relay                           | Readable diagnostic; fail-open default (original content preserved)                                   |
| Multiple registered handlers | N/A for tools (single-name dispatch) - applies to hooks        | Covered by `aphrodite-hook-reference`                                                                 |
| Registered but never invoked | Compare `plugin.yaml` tool list vs the live tool catalog       | Every registered tool invocable; orphans flagged                                                      |
| Restarted process            | Fresh Hermes session after a dylib change                      | Version handshake matches `BINARY_VERSION`; stale dylib excluded; store re-verified via stats/catalog |

## Proxy Architecture (context)

- **Token proxy** (token listener - port is a config property, default
  `:9798`; read live from `aphrodite.toml` `ports`, never assume) -
  token-level compression; management endpoints require an API key.
- **Cache proxy** (cache listener - port is a config property, default
  `:9797`; read live) - cache-mode compression; management endpoints accept
  any loopback caller.
- Both share the CCR database at `~/.hermes/aphrodite/ccr.db`.
- Binary: `~/.hermes/aphrodite/binaries/aphrodite` (auto-updated).
- Runtime home layout: `~/.hermes/aphrodite` holds binaries/, `aphrodite.toml`,
  the BINARY_VERSION pin, `ccr.db`, directives/, and logs/;
  `~/.hermes/plugins/aphrodite` holds ONLY `plugin.yaml` + `__init__.py`
  (hooks-only plugin install; `aphrodite setup` writes both).
- Dylib hot-reloads on file modification.

**Confidence:** the two ports are runtime-derived - read them live from the
active config, never assume the defaults.

## Checklist for Agents

- [ ] See `<<<CCR:hash|type|size>>>` -> `aphrodite_retrieve(hash)` before any
      other action; never re-read the source file behind the marker
- [ ] Resolve all markers the next action needs in the same turn (local tools
      one call per invocation; batch connector tools)
- [ ] Prefer a `query` filter over full content for large entries
- [ ] Retrieval fails (`found: false`) -> fall back to `read_file` / `terminal`
- [ ] Never compress retrieval or diagnostic responses
- [ ] `aphrodite_test(mode="quick")` to verify engine health
- [ ] `aphrodite_stats()` to check proxy health before relying on CCR tools
- [ ] `aphrodite_prefetch(paths=[...])` to batch-read 3+ files in background
- [ ] Keep skill content/templates benign - skills are hash-scanned on load
      (skills_guard); flagged content quarantines the skill until a re-scan
- [ ] Parallel verification sessions on free-tier providers hit intermittent
      HTTP 401/429 - re-dispatch failed clusters smaller; treat as transient
      until a cluster fails repeatedly

## Local test matrix

| Claim                                  | Evidence source                 | Test                                                | Pass condition                                      | Failure response                                   |
| -------------------------------------- | ------------------------------- | --------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------- |
| 13-tool inventory matches registration | `plugins/aphrodite/plugin.yaml` | Diff the inventory table vs the `tools` list        | Identical names                                     | Update this skill; report drift to manifest owner  |
| Retrieve-first prevents content loss   | Session CCR engine              | Compress payload, emit marker, resolve via retrieve | Resolved bytes equal source                         | Fix resolver/plugin path                           |
| `hash` required for retrieve           | `proxy.rs` validation           | `aphrodite_retrieve(query only)`                    | `{found: false, error}`                             | Update tool contract/test                          |
| Retrieval responses never recompressed | Transform pipeline              | Feed a resolved payload through the transform       | Payload stays raw; no nested marker                 | Update skip classifier (owner: compression-safety) |
| Read-only tools never mutate           | Proxy/store                     | Run each read-only tool twice                       | Identical results; no new CCR entries               | Reclassify the mutating tool                       |
| Auto-expand is inert                   | Config consumer scan            | Set TOML/env knob; observe marker behavior          | No auto-resolution occurs; retrieve still canonical | Update `aphrodite-auto-expand-testing`             |
