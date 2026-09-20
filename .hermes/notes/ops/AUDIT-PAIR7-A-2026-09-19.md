# AUDIT-PAIR7-A-2026-09-19 - stale-content audit (part A)

Repo: PlayForm/Aphrodite (branch Development, binary 1.4.6, plugin 2.1.4, default branch Current).
Audit: READ-ONLY. No docs/ file modified. Evidence verified against source on 2026-09-19.
Scope: docs/agent-feedback.md (379 lines), docs/APHRODITE-HEADROOM.md (159 lines).
Partner (B) audits docs/HEADROOM-FORK-DIFF.md + docs/hermes-tool-output-schemas.md + docs/centers.md (LANDED deleg_b7c7d65c).

## Verdict summary

| File                       | Verdict       | Destination                                        |
| -------------------------- | ------------- | -------------------------------------------------- |
| docs/agent-feedback.md     | MOVE-INTERNAL | .hermes/notes/agent-feedback.md (internal archive) |
| docs/APHRODITE-HEADROOM.md | MOVE-INTERNAL | .hermes/notes/APHRODITE-HEADROOM.md (internal)     |

Rationale (both): fork-divergence/feedback docs were already removed from the release
line in 1.4.6 (.hermes/release-notes/v1.4.6.md:54-56); internal process artifacts are
structural, not cosmetic; both retain engineering value -> archive, do not delete.

---

## 1. docs/agent-feedback.md (379 lines) -> MOVE-INTERNAL

Destination: `.hermes/notes/agent-feedback.md` (flat .hermes/notes/ archive, same
convention as partner B's .hermes/notes/HEADROOM-FORK-DIFF.md).

### INTERNAL evidence (feedback-note artifact by its own header)

- L3: "**Date:** 2026-07-31 ... **Scope:** Generalized feedback for agents using the
  Aphrodite CCR compression engine ... Adapted from session feedback collected across
  multiple projects (Aphrodite, STE-Code) and stripped of all user-specific paths" -
  explicit feedback-note header; internal artifact framing.
- L7: "CCR Marker Handling - The #1 Thing Agents Get Wrong" - feedback tone.
- L47-52: "Consequences of ignoring CCR: You respond with 'I got compressed output'
  ... This is the #1 cause of poor agent performance" - session-derived feedback prose.
- Sections 6 (Background Process Management, L146-162), 7 (API Parallelism Limits,
  L166-181), 8 (File Editing Best Practices, L185-202), 9 (Git Workflow for Generated
  Files, L206-217), 14 (Quality Gate Enforcement, L360-368) - agent-operational and
  dev-workflow lessons (2-way API concurrency, sed pitfalls, gitignore negation, retry
  gates), not product functionality; internal by nature.
- L54: "Retrieval is cheap (sub-ms from local SQLite). Thinking/acting on the content
  is what costs tokens." - performance coaching note.

### STALE evidence (against verified source)

- L223 + L230 + L244: "Aphrodite ships five built-in directives baked into the binary
  via include_str!" / table lists 5 / "the 5 baked-in directives" - SEVEN are baked in:
  crates/aphrodite/src/directives.rs:27-33 (focus, foresight, ccr-handling, cleanup,
  explore, lazy, lazy-eval; table also omits lazy + lazy-eval). STALE.
- L336: "code | x4 (default) | 4,096" - shipped default is code_multiplier = 3.0
  (aphrodite.toml.example:63), applied at proxy.rs:492-493; old env-var default was 2
  (proxy.rs:461-465). STALE (x4 not a real default anywhere).

### Verified-OK evidence (no action needed in rewrite if preserved)

- L15: BLAKE3 hash, 40 hex chars - hooks.rs:28-29 (compute_hash -> headroom_core::ccr::compute_key, "BLAKE3 (40 hex chars)").
- L110: auto_expand = true is the shipped default - aphrodite.toml.example:59.
- L238: directives search order APHRODITE_DIRECTIVES_DIR -> ./directives/ ->
  ~/.hermes/aphrodite/directives/ -> binary-relative, first existing wins, not merged -
  config_loader.rs:211-231.
- L241-242: per-file cap 2,000 chars / combined cap 4,000 - directives.rs:62,68.
- L326-328: tier table (inline <256B LruCache 1,024 entries; cache >8KB InMemoryCcrStore
  10,000; token >1KB SQLite) - matches fresh docs: docs/ccr/backends/inline.md:17,22,
  docs/ccr/backends/in-memory.md:36,39, docs/proxy/architecture.md:20-21 (8192/1024 byte
  thresholds, 10,000 entries, ccr_ttl_seconds default 3600).
- L340-342: linter/build_output/log pinned at BASE, not halved - proxy.rs:475.
- L352-354: runtime home ~/.hermes/aphrodite (ccr.db, binaries/) - **init**.py:34,145,317,
  501,519,801,919; ccr.db joined at proxy.rs:670.
- L290-314: first-turn session_inject from [prompts] session_inject, SHIPPED_SESSION_INJECT
  fallback - config_loader.rs:293-301, flow.rs:18.
- L375-379 "Related Resources" - all five targets exist post-rewrite
  (docs/plugin/hooks.md, docs/plugin/directives.md, docs/ccr/lifecycle.md,
  docs/tool-relay/tools.md, docs/config/aphrodite-toml.md).

### Action for orchestrator

`git mv docs/agent-feedback.md .hermes/notes/agent-feedback.md` (or plain move + commit
sweep). Optionally prune sections 1-5/10-13 inside the archive (superseded by fresh
docs/ccr, docs/plugin, docs/tool-relay, docs/config); sections 6-9/14 are unique
internal ops knowledge - keep.

---

## 2. docs/APHRODITE-HEADROOM.md (159 lines) -> MOVE-INTERNAL

Destination: `.hermes/notes/APHRODITE-HEADROOM.md` (internal archive, mirroring partner
B's .hermes/notes/HEADROOM-FORK-DIFF.md; the two fork-divergence docs stay together).

### INTERNAL evidence

- .hermes/release-notes/v1.4.6.md:54-56: fork-divergence docs (`APHRODITE-HEADROOM.md`,
  `HEADROOM-FORK-DIFF.md`, ...) "removed from the release line" in 1.4.6 - product
  already de-listed this doc from public distribution.
- L122-155 "Updating the vendored fork": internal pin-bump checklist (merge upstream,
  fork test suite, one-commit-per-bump, serde_json feature-parity check) - engineering
  process, not user documentation.
- L125: "(report 08 F10/T7)", L141: "(report 08 F2)", L154: "(report 08 F3)" - internal
  report references.
- L142-143: cross-references `HEADROOM-FORK-DIFF.md`'s 2026-07-11 merge section - target
  is moving internal per partner B; public link would dangle.
- L34-36: submodule/fork maintenance internals (vendor/headroom, PlayForm/Headroom fork,
  chopratejas upstream).

### STALE evidence (against verified source)

- L16 + L45: "26-type classifier" / "Extended from generic to 26 typed categories" -
  pipeline emits 30 distinct content types: docs/classification/content-types.md:10,118.
  STALE (PAIR-2-D confirmed 30 not 26).
- L97: "5 lifecycle hooks (on_session_start, transform_tool_result, ...)" - SIX hooks
  dispatched: crates/aphrodite-hermes/src/lib.rs:335 (on_session_start), 336
  (pre_tool_call), 409 (transform_tool_result), 442 (transform_terminal_output), 458
  (pre_llm_call), 477 (post_llm_call). STALE.
- L19: "Auto-expand ... (off by default)" - shipped config sets auto_expand = true
  (aphrodite.toml.example:59). STALE.
- L41: "Python side uses SHA-256 independently - hashes differ but are consistent within
  each language" - FALSE: zero hashing code in the Python plugin (0 "hash" matches in
  plugins/aphrodite/**init**.py; _bindings.py FFI exposes no hash symbol; sha256/blake3/
  hashlib only in download.sh binary-integrity check + layout_check.py). CCR hash is Rust
  BLAKE3 40-hex via headroom_core (hooks.rs:28-29). Also self-contradicts L28 of the same
  doc ("Identical CCR hash, marker format, inline store between Rust proxy and Python
  plugin"). STALE.
- L85: "Location: ~/.hermes/aphrodite/aphrodite" - canonical path is
  ~/.hermes/aphrodite/binaries/aphrodite (**init**.py:317,343,501,519,801,919). STALE.
- L25: "Rhai scripting ... (--features scripting)" - no rhai/scripting in any crates/*
  Cargo.toml or Cargo.lock (grep: zero matches). Feature absent in 1.4.6. STALE.
- L159: links `.../tree/Development/docs/HEADROOM-FORK-DIFF.md` - default branch is
  Current (remotes/Source/HEAD -> Source/Current; git branch -a). Violates the
  tree/Development link rule; target file is moving internal anyway. STALE.

### Verified-OK evidence (in case any table row is salvaged)

- L20 + L49: "28 Prometheus metrics" - docs/metrics/prometheus.md:20 ("28 distinct metric
  names, matching the /metrics output name-for-name"). OK.
- L65: "25 C ABI functions" - 25 `pub extern` in crates/aphrodite/src/lib.rs (grep count). OK.
- L27: fork repository URL github.com/PlayForm/Headroom - vendor/headroom/Cargo.toml:27. OK.
- L12/98: 13 tools - verified by PAIR-5 (docs/tool-relay/tools.md).
- L64: dual proxy :9797 + :9798 - crates/aphrodite/src/main.rs:4,805-806. OK.
- L24: APHRODITE_LIVE_CONTAINER mode - documented in fresh docs/config/env-vars.md. OK.

### Action for orchestrator

`git mv docs/APHRODITE-HEADROOM.md .hermes/notes/APHRODITE-HEADROOM.md` (or plain move +
commit sweep). Keep paired with .hermes/notes/HEADROOM-FORK-DIFF.md.

---

## Discrepancy log (for orchestrator to append to DOC-REWRITE-PUBLIC-2026-09-19.md)

- [2026-09-19] docs/agent-feedback.md:223,230,244: claimed FIVE built-in directives;
  directives.rs:27-33 bakes in SEVEN (focus, foresight, ccr-handling, cleanup, explore,
  lazy, lazy-eval).
- [2026-09-19] docs/agent-feedback.md:336: code type threshold multiplier "x4 (default)";
  shipped code_multiplier = 3.0 (aphrodite.toml.example:63; proxy.rs:492-493).
- [2026-09-19] docs/APHRODITE-HEADROOM.md:16,45: "26-type classifier"; pipeline emits 30
  distinct content types (docs/classification/content-types.md:10,118).
- [2026-09-19] docs/APHRODITE-HEADROOM.md:97: "5 lifecycle hooks"; SIX dispatched
  (crates/aphrodite-hermes/src/lib.rs:335,336,409,442,458,477).
- [2026-09-19] docs/APHRODITE-HEADROOM.md:19: auto-expand "off by default"; shipped
  auto_expand = true (aphrodite.toml.example:59).
- [2026-09-19] docs/APHRODITE-HEADROOM.md:41: "Python side uses SHA-256 independently";
  no hashing code in plugins/aphrodite/ (0 hash matches in **init**.py); CCR hash is Rust
  BLAKE3 (hooks.rs:28-29); contradicts the doc's own L28 parity claim.
- [2026-09-19] docs/APHRODITE-HEADROOM.md:85: binary at ~/.hermes/aphrodite/aphrodite;
  real path ~/.hermes/aphrodite/binaries/aphrodite (**init**.py:317,343,501,519,801,919).
- [2026-09-19] docs/APHRODITE-HEADROOM.md:25: Rhai scripting feature-gated hooks; no rhai/
  scripting in crates Cargo.toml or Cargo.lock (absent in 1.4.6).
- [2026-09-19] docs/APHRODITE-HEADROOM.md:159: tree/Development link; default branch is
  Current (remotes/Source/HEAD -> Source/Current).
