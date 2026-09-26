---
name: aphrodite-tool-testing
description: "Use when teaching or verifying aphrodite CCR tool usage: the 13 CCR tools, marker handling, the retrieve-first rule, read-only vs state-changing classification, and the tool test cases."
version: 2.2.0
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
    - 13-tool CCR inventory (table in SKILL.md; behavior notes in references/per-tool-behavior.md)
    - Retrieve-first rule (marker -> aphrodite_retrieve before any other action)
    - Read-only vs state-changing classification of CCR tools
    - CCR tool test cases (references/tool-test-cases.md)
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
- Checking whether the CCR engine is healthy (`aphrodite_test(mode="quick")`),
  or teaching agents the 13-tool API.

## The 13 CCR Tools (Quick Reference)

This table is source-derived, not a standing claim. The registration source of
truth is `plugins/aphrodite/plugin.yaml` (lines 15-27); the tool-relay
implementation is `crates/aphrodite/src/proxy.rs`. Before teaching the table,
diff it against `plugin.yaml`'s `tools` list and the live session tool catalog.
If the two differ, update this skill and report the drift to the manifest
owner.

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
`aphrodite_debug`, which toggles per-session debug output on/off (Rust-side
only). It is NOT registered in `plugin.yaml`. This note is runtime-derived:
diff against the live tool catalog before relying on it. `aphrodite_debug` is
not part of the registered 13.

## Retrieve-first rule

**A CCR marker in tool output is content. It is not a placeholder.** When a
marker appears, resolve it with `aphrodite_retrieve(hash)` before any other
action that consumes the content it stands for. This is the standing rule in
every session. Auto-expand is inert configuration; it never resolves markers
for you (see `aphrodite-auto-expand-testing`).

- Never re-read the source file behind a marker with `read_file`, because the
  marker IS the content; re-reading just yields another marker.
- Resolve EVERY marker the next action needs, in the same turn. Local tools
  take one call per invocation: a `tool_call` with multiple local entries is
  rejected. Issue the retrieves one at a time and complete them all before
  acting. Connector tools may be batched.
- Prefer a `query` filter over full content when only matching lines are
  needed, because retrieval is cheap and filtering is cheaper.
- CLAIM: nested markers are expanded recursively by the resolver.
- Never compress a retrieval response or an Aphrodite diagnostic response,
  because this is a context boundary (canonical owner:
  `aphrodite-compression-safety`). Resolving a marker is non-destructive and
  never re-marks the payload.

Why agents fail:

1. They treat the marker as opaque and lose the content it stands for.
2. They re-read the source file with `read_file` instead of retrieving.
3. They retrieve one marker at a time across turns instead of resolving all
   needed markers in the same turn.
4. They rely on auto-expand or "the preview was fine" instead of the canonical
   retrieval path.

## Using `aphrodite_retrieve`

1. **Full content by hash**: `aphrodite_retrieve(hash="abc123...")` ->
   `{found: true, source: "ccr", hash: "...", content: "..."}`.
2. **Query-filtered**: `aphrodite_retrieve(hash="abc123...", query="timeout")`
   returns only lines matching the query.
3. **Direct file read (fallback)**: `aphrodite_retrieve(path="README.md")` ->
   `{found: true, source: "path", ...}`. Path-based reads enforce workspace
   containment: `aphrodite_retrieve(path="...")` on a path outside the
   workspace returns `found: false`.
4. **When retrieval fails**: if `aphrodite_retrieve(hash=...)` returns
   `found: false`, fall back to `read_file` / `terminal` on the original path.
   The failed hash is a resolver gap; it is not a reason to invent content.

## Read-only vs state-changing

Read-only tools (`stats`, `search`, `catalog`, `diff`, `files`,
`prefetch_status`, `rebuild`, `retrieve`) never mutate session or store state.
Running them twice yields identical results and no new CCR entries.
State-changing tools (`compress`, `test`, `reclassify`, `prefetch`,
`directive`, `debug`) write entries, mutate metadata, or change session
behavior. Treat each state-changing call as deliberate. A verification flow
that mutates is not a read-only verification.

## Never compress retrieval/diagnostic responses

Retrieval, catalog, status, and health results are never compressible. This is
a context boundary (canonical owner: `aphrodite-compression-safety`, per the
skill manifest). Re-emitting a CCR marker for a value that is already a
resolved retrieval payload is forbidden, because a retrieval response stays raw
through the transform pass. If a retrieval response ever comes back as a
marker, that is a bug in the skip classifier; it is not a reason to resolve
again.

## Stop-if / Recovery

- **Stop if** `aphrodite_retrieve(hash=...)` returns `found: false` for a hash
  you hold a marker for. The resolver cannot reach the stored content.
- **Recovery** fall back to `read_file` / `terminal` on the original path and
  record the failed hash; never invent content you could not see.
- **Stop if** `aphrodite_test(mode="quick")` returns anything other than
  `status="ok"`. The compress -> retrieve round trip is broken.
- **Recovery** run `aphrodite_stats()` and check proxy health before relying on
  any CCR tool; fix the proxy before resuming marker work.

## Checklist for Agents

- [ ] See `<<<CCR:hash|type|size>>>` -> `aphrodite_retrieve(hash)` before any
      other action; never re-read the source file behind the marker, because
      the marker IS the content; re-reading yields another marker
- [ ] Resolve all markers the next action needs in the same turn (local tools
      one call per invocation; batch connector tools)
- [ ] Prefer a `query` filter over full content for large entries
- [ ] Retrieval fails (`found: false`) -> fall back to `read_file` / `terminal`
- [ ] Never compress retrieval or diagnostic responses, because a retrieval
      response stays raw through the transform pass
- [ ] `aphrodite_test(mode="quick")` to verify engine health
- [ ] `aphrodite_stats()` to check proxy health before relying on CCR tools
- [ ] `aphrodite_prefetch(paths=[...])` to batch-read 3+ files in background
- [ ] Keep skill content/templates benign - skills are hash-scanned on load
      (skills_guard); flagged content quarantines the skill until a re-scan
- [ ] Parallel verification sessions on free-tier providers hit intermittent
      HTTP 401/429 - re-dispatch failed clusters smaller; treat as transient
      until a cluster fails repeatedly

## References

- `references/tool-test-cases.md` - the per-tool test case matrix, adapted
  from the hook test cases.
- `references/per-tool-behavior.md` - per-tool behavior notes for the 13 tools
  plus `aphrodite_debug`.
- `references/proxy-architecture.md` - proxy architecture, ports, CCR store,
  binary, and runtime home layout.

## Local test matrix

| Claim                                  | Evidence source                 | Test                                                | Pass condition                                      | Failure response                                   |
| -------------------------------------- | ------------------------------- | --------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------- |
| 13-tool inventory matches registration | `plugins/aphrodite/plugin.yaml` | Diff the inventory table vs the `tools` list        | Identical names                                     | Update this skill; report drift to manifest owner  |
| Retrieve-first prevents content loss   | Session CCR engine              | Compress payload, emit marker, resolve via retrieve | Resolved bytes equal source                         | Fix resolver/plugin path                           |
| `hash` required for retrieve           | `proxy.rs` validation           | `aphrodite_retrieve(query only)`                    | `{found: false, error}`                             | Update tool contract/test                          |
| Retrieval responses never recompressed | Transform pipeline              | Feed a resolved payload through the transform       | Payload stays raw; no nested marker                 | Update skip classifier (owner: compression-safety) |
| Read-only tools never mutate           | Proxy/store                     | Run each read-only tool twice                       | Identical results; no new CCR entries               | Reclassify the mutating tool                       |
| Auto-expand is inert                   | Config consumer scan            | Set TOML/env knob; observe marker behavior          | No auto-resolution occurs; retrieve still canonical | Update `aphrodite-auto-expand-testing`             |
