# Headroom Compressor Comparison

Discovered 2026-06-17 while analyzing whether Aphrodite could use Headroom's semantic compressors for code reduction and navigability.

## Headroom's 9 Compression Strategies

| Strategy | Class | Target | Technique | Ratio |
|----------|-------|--------|-----------|-------|
| `CODE_AWARE` | `CodeAwareCompressor` | Source code | tree-sitter AST — keeps imports/sigs/types, drops bodies to `# ...` placeholders | 5–8× |
| `SMART_CRUSHER` | — | JSON arrays | Structural dedup of repeated array elements | — |
| `SEARCH` | `SearchCompressor` | grep/ripgrep results | Dedup + summarize match lines | — |
| `LOG` | `LogCompressor` | Build/test output | Extract errors/warnings, drop compilation noise | — |
| `KOMPRESS` | `KompressCompressor` | Free text | ML-based semantic compression (nn.Module) | 3–5× |
| `DIFF` | `DiffCompressor` | Git diffs | File-level summary, drop hunks | — |
| `HTML` | — | Web content | Tag-aware structural compression | — |
| `MIXED` | `ContentRouter` | Chat/tool output | Split → route per section → reassemble | — |
| `PASSTHROUGH` | — | Below threshold | No-op identity | 1× |

Routing via `ContentRouter` (`headroom/transforms/content_router.py`, 2976 lines):
1. Uses source hint if available (highest confidence)
2. Checks for mixed content (code fences + JSON + prose)
3. Detects content type via regex classifier
4. Routes to appropriate compressor
5. Reassembles with routing metadata

## Aphrodite Today

One "compressor": SHA-256 → store in SQLite/in-memory → return `<<<CCR:hash|type|size>>>`. Content-addressed deduplication, zero semantic reduction. Ratio: 1× (identity).

## Integration Opportunities

### Code Reduction (highest impact)
`CodeAwareCompressor` uses tree-sitter to parse code into AST, selectively compress function bodies while preserving structure. Output is syntactically valid. Key advantages:
- Syntax validity guaranteed
- Preserves imports, signatures, types
- 5–8× ratio vs 3–5× for token-level compression
- Lower latency (20–50ms vs 50–200ms)
- Smaller memory (~50MB vs ~1GB)
- Thread-safe (thread-local tree-sitter parsers)

Integration path: Aphrodite proxy accepts `strategy=ast` param → calls CodeAwareCompressor → stores reduced content in CCR. LLM gets semantically-compressed blob with preserved structure.

Example transformation:
```rust
// Before (10 lines)
fn process(items: &[String]) -> Vec<String> {
    let mut results = Vec::new();
    for item in items {
        if item.is_empty() { continue; }
        results.push(item.trim().to_lowercase());
    }
    results
}

// After (2 lines) — LLM still sees the contract
fn process(items: &[String]) -> Vec<String> {
    // ... (body compressed: 8 lines → placeholder)
}
```

### Navigability (structure maps)
Current CCR previews: `[code:fn process... 30L]` — opaque. LLM must retrieve full blob to know contents. Better: extract function/struct/impl list as preview so LLM can browse without retrieving:

```
[code:3fns|2structs|1impl proxy.rs]
  fn format_ccr_output(preview, ct, metadata, center, hash, size) -> String
  fn build_preview(content, ct) -> String
  fn smart_marker(hash, content, ct, center) -> String
```

No tree-sitter needed — just regex for `fn`, `struct`, `impl`, `class`, `def` patterns. LLM sees the index and retrieves only what it needs.

### Split+Route for Mixed Content
Tool output is mixed content. `cargo build` produces compilation lines + error blocks + JSON. ContentRouter splits, routes each section to the right compressor, reassembles. Aphrodite currently treats the whole thing as one monolithic blob.

## Key Source Files
- `vendor/headroom/headroom/transforms/content_router.py` — router + 9 strategies (2976 lines)
- `vendor/headroom/headroom/transforms/code_compressor.py` — `CodeAwareCompressor` (2036 lines)
- `vendor/headroom/headroom/transforms/search_compressor.py` — `SearchCompressor`
- `vendor/headroom/headroom/transforms/log_compressor.py` — `LogCompressor`
- `vendor/headroom/headroom/transforms/diff_compressor.py` — `DiffCompressor`
- `vendor/headroom/headroom/transforms/kompress_compressor.py` — `KompressCompressor` (ML-based)
