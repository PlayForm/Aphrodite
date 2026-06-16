---
name: aphrodite-center-testing
description: "Test aphrodite center features end-to-end — call site audit, orphaned function detection, preview depth verification across all compression paths."
version: 1.0.0
platforms: [macos]
related_skills: [aphrodite-dev-workflow, aphrodite-hook-reference]
---

# Aphrodite Center Feature Testing

Centers are temporary LLM memory deposits that travel with CCR markers (v0.5.84+). Defined in `crates/aphrodite/src/center.rs`. The LLM sets a center NOW, content arrives LATER. They embed in the structure line of multi-line CCR markers.

## Multi-Line Format

```
<preview line>
[type: metadata;center=code_rust]
<<<CCR:hash|type|size>>>
```

Single-line legacy format (tool output hooks):
```
<<<CCR:hash|type|size|token>>>
```

## 5-Phase Testing Methodology

### Phase 1: Code Audit
1. Read `center.rs` — note expected behavior (preview depth 100/250/500, content type override)
2. Search ALL callers of `parse_center`, `centered_content_type`, `centered_preview_len`
3. ZERO callers → orphaned dead code → report
4. Check `build_preview()` — does it accept `center`? (it doesn't)
5. Check `smart_marker()` / `cache_marker()` — do they pass center through to build_preview? (they don't)

### Phase 2: Call Site Matrix

| Path | File | Center? |
|---|---|---|
| aphrodite_compress tool relay | proxy.rs execute_tool_relay | YES |
| Chat Completions API | proxy.rs compress_chat_completion | None hardcoded |
| Tool hooks (proxy) | _hooks.py | NO |
| Tool hooks (inline) | _hooks.py | NO |
| Context engine | _engine.py | NO |
| /ccr/create endpoint | proxy.rs | NO |

### Phase 3: Behavioral Test
1. Compress same content × all centers: code_rust, code_python, debug, verbose, compact, summary, None, random
2. Measure preview depth — expected: debug=500, compact=100, default=250, code=300
3. Verify `;center=X` in structure line
4. Retrieve + verify content integrity
5. Check `_parse_ccr_markers()` extracts center (it doesn't)

### Phase 4: Python → Rust Header
1. Python reads `_ccr_center` from args — YES (v0.5.86)
2. COMPRESS_SCHEMA exposes it — YES
3. `X-Aphrodite-Center` header sent — YES
4. Rust `/ccr/create` reads header — NO

### Phase 5: Format Sync
1. Python `_ccr_marker()` has `center=None` param — YES (v0.5.86)
2. Structure line appends `;center=X` — YES
3. Format matches Rust `format_ccr_output()` — YES
4. `_parse_ccr_markers()` handles center — NO

## Known Gaps (v0.5.90)

All three center.rs functions are orphaned:
- `parse_center()` — 0 callers
- `centered_content_type()` — 0 callers
- `centered_preview_len()` — 0 callers

Consequences:
- Preview depth never varies by center type
- Content type is never overridden by center
- Only 1 of 7 compression paths passes center
- Hooks/engine never use centers
- `/ccr/create` ignores `X-Aphrodite-Center` header
- `_parse_ccr_markers()` drops center on re-compression
- Retrieval doesn't return center metadata

What works: center travels in structure line for display to LLM. No behavioral effect.
