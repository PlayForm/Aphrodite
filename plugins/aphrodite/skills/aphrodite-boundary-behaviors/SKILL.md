---
name: aphrodite-boundary-behaviors
description: "Boundaries between aphrodite compression layers: retrieve tool vs engine auto-expand, proxy vs inline storage, cross-proxy routing, center annotation vs storage. Testing and debugging these seams."
version: 2.0.0
platforms: [macos]
related_skills: [aphrodite-tool-guide, aphrodite-dev-workflow]
---

# Aphrodite Boundary Behaviors

Knowledge about the boundaries (seams) between the three aphrodite compression layers:
1. **Proxy** (Rust binary, :9797 cache / :9798 token) — stores CCR, produces markers
2. **Python Plugin** (context engine, inline store) — auto-resolves markers (when enabled), provides tools
3. **Hermes Agent** (LLM) — sees markers, calls tools, triggers compression

Understanding these boundaries is essential for testing and debugging why content appears or doesn't appear.

## Boundary 1: Retrieve Tool vs Engine Auto-Expand

This is the most commonly confused boundary.

| Aspect | `aphrodite_retrieve` tool | Engine auto-expand |
|--------|--------------------------|-------------------|
| Who triggers | LLM calls it explicitly | Plugin's `_pre_llm_hook` runs automatically |
| When | On demand, any turn | Before each LLM turn |
| Controlled by | N/A (always works) | `AUTO_EXPAND_LIMIT` from TOML `auto_expand_limit` (default 5 = effectively OFF). Set `APHRODITE_AUTO_EXPAND=1` to enable (limit=51200). |
| Returns | Full expanded content | Full content inlined into context |
| Marker removal | Marker stays in conversation | Small markers (< limit) resolved in-place |

**Source**: `_hooks/session.py:155-189`, `_core/config.py:161-163`. `APHRODITE_NO_AUTO_EXPAND` does NOT exist in source — it was fictional in old skills.

**Testing rule**: Auto-expand is effectively OFF by default (limit=5 bytes). Calling `aphrodite_retrieve` always returns full content. To test auto-expand behavior, examine what the LLM sees in its context (raw markers vs expanded).

## Boundary 2: Proxy vs Inline Storage

| Store | Location | Persistence | Scope |
|-------|----------|-------------|-------|
| Cache proxy (:9797) | In-memory LRU (10K entries) | Process lifetime | Cache profile only |
| Token proxy (:9798) | SQLite at `~/.hermes/aphrodite/ccr.db` | Persistent across restarts | Token profile only |
| Python inline store | Plugin memory (zlib-compressed) | Session lifetime | Current agent+workers |

**Cross-store routing**: The Python plugin routes `aphrodite_retrieve` through the token proxy (:9798). Content stored via `/ccr/create` on :9798 is retrievable from the Python tool. But the cache proxy (:9797) has its OWN store — content there is NOT visible to the token proxy and vice versa.

## Boundary 3: Center Annotation vs Storage

The `_ccr_center` parameter (e.g., `code_rust`) travels with the CCR marker in its format string (`;center=X`) but has **zero effect on storage**:

- `compute_key()` hashes content-only — same content with different centers produces the same hash
- `ccr_put()` stores bare content — center is discarded
- Retrieval returns original content — center is not stored or restored
- The center survives only in the marker string produced by `format_ccr_output()`

**Testing implication**: Changing the center of identical content produces the same hash and overwrites the previous entry. Centers are annotations for the LLM's benefit, not storage keys.

## Boundary 4: Worker Isolation

Subagents (delegate_task workers) are separate sessions with their own:
- Python inline store (content compressed by Worker A is invisible to Worker B)
- Terminal/file sessions
- Proxy connections (shared — both hit the same :9797/:9798)

**Cross-worker sharing**: To share compressed content between workers:
1. Compress via `aphrodite_compress()` (hits shared proxy SQLite)
2. Pass the hash in worker `context`
3. Worker resolves with `aphrodite_retrieve(hash)`

## Boundary 5: Compression Thresholds

Content only crosses the proxy→marker boundary when it exceeds the type-specific threshold:

| Mode | Base Threshold | code_rust multiplier | Effective code_rust threshold |
|------|---------------|---------------------|-------------------------------|
| Cache (:9797) | 8KB (8192) | 4× | ~32KB (8192 × 4) |
| Token (:9798) | 1KB (1024) | 4× | ~4KB (1024 × 4) |

Plus headroom budget modulation: budget_mult ∈ [0.50, 1.0] from `x-headroom-budget` header.

**Testing implication**: Content just above base threshold but below type-adjusted threshold will NOT be compressed. Always calculate `threshold_for(ct) × budget_mult` before expecting compression.

## Testing Workflow

```python
# 1. Check what's alive
aphrodite_stats()

# 2. Compress via proxy API (direct, no tool)
# Use curl to :9798/ccr/create with the content

# 3. Verify retrieval works
aphrodite_retrieve(hash="<hash>")

# 4. Check auto-expand behavior
# Run a >threshold command, check next turn for raw markers
```

## Related

- `aphrodite-tool-guide` — tool reference, master-worker pattern, common pitfalls
- `aphrodite-dev-workflow` — release pipeline, profile switching, build monitor
- `aphrodite-auto-expand-testing` — full auto-expand test protocol
