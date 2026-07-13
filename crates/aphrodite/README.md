# aphrodite 💋 Core Engine

> **CCR compression proxy + cdylib - classify, compress, store, preview.**
> **Sub-ms, 28 content types, 12,800× max ratio.**

The core compression engine. Produces both the `aphrodite` binary (HTTP proxy on
`:9797`/`:9798`) and `libaphrodite.dylib` (loaded by the Hermes plugin via C ABI).

[crates.io](https://crates.io/crates/aphrodite) ·
[docs](../../docs/README.md)

---

## Install

```bash
# From source (monorepo)
cargo build --release -p aphrodite

# From crates.io
cargo install aphrodite
```

---

## What it does

```
tool output → classify → template → store → <<<CCR:hash|type|size>>>
                                              │
                                              ▼
                                         Agent (15 tok preview, not 500 tok raw)
```

Four pipeline stages, all under 1ms:

1. **Classify** - 28-type regex classifier (`diff`, `build_output`, `code_rust`, …)
2. **Template** - TOML-driven preview templates per content type
3. **Store** - blake3 hash → SQLite or in-memory → CCR marker
4. **Preview** - Structured `[type:metadata]` the LLM reads instantly

---

## Architecture

```
src/
├── lib.rs               ← 22 C ABI exports for dylib loading
├── proxy.rs             ← HTTP proxy (:9797 cache, :9798 token)
├── hooks.rs             ← transform_tool_result, terminal, pre/post LLM
├── session.rs           ← Turn lifecycle, conversation index, catalog
├── state.rs             ← AphroditeState, inline store, LRU, markers
├── marker.rs            ← CCR marker formatting + parsing (<<<CCR:…>>>)
├── catalog.rs           ← Full/compact/TOC catalog display
├── resolve.rs           ← Recursive CCR expansion (3 levels deep)
├── stage2.rs            ← Semantic reduction for JSON, build, diff, code
├── struct_extract.rs    ← Code structure maps (Rust, Python, Go, JS/TS)
├── config.rs            ← CLI args + TOML multi-config
├── config_loader.rs     ← TOML + env var loading
├── prefetch.rs          ← Background file read + compress threads
├── preview.rs           ← detect_type, build_preview (shared across proxy + dylib)
├── main.rs              ← Binary entry point
└── retrieve.rs          ← POST /retrieve — hash → content, zstd decode, filter, paginate
```

## C ABI (22 functions)

The 22 `#[no_mangle] extern "C"` exports in `lib.rs` serve both the
`aphrodite-hermes` bridge crate (which re-exports a higher-level ABI — see
`crates/aphrodite-hermes/README.md`) and the Python plugin's ctypes bindings.

---

## Dependencies

- `headroom-core` - Content transforms + classifier + SQLite backend (vendored fork at `vendor/headroom/`)
- `axum` / `tokio` / `tower-http` - HTTP proxy (optional, gated behind `proxy` feature)
- `serde` / `serde_json` - Serialization
- `blake3` - Content-addressed hashing

---

## See Also

- [Installing Aphrodite](../../docs/install/README.md) - which artifact you
  need, per-platform install guides, troubleshooting
- [aphrodite.toml Configuration](../../docs/config/aphrodite-toml.md) - full
  TOML schema this crate's `config.rs` deserializes
- [Hermes Integration](../../docs/hermes-integration.md) - how this binary's
  sibling dylib crate (`aphrodite-hermes`) plugs into Hermes Agent

## License

CC0-1.0 - public domain.
