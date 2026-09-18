# aphrodite 💋 Core Engine

> **CCR compression proxy + cdylib - classify, compress, store, preview.**
> **Sub-ms, 26 content types, 12,800× max ratio.**

The core compression engine. Produces both the `aphrodite` binary (HTTP proxy on
`:9797`/`:9798`) and `libaphrodite.dylib` (best-effort cdylib for external
embedding; the Hermes plugin loads the bridge crate's `libaphrodite_hermes.dylib`
instead - see `crates/aphrodite-hermes/README.md`).

[crates.io](https://crates.io/crates/aphrodite) ·
[docs](https://github.com/PlayForm/Aphrodite/tree/Development/docs/README.md)

---

## Install

```bash
# From source (monorepo)
cargo build --release -p aphrodite
# then bootstrap the runtime home (~/.hermes/aphrodite/: binaries/,
# directives/, hotreload/, ccr.db)
aphrodite setup

# From crates.io
cargo install aphrodite
```

---

## What it does

```
tool output → classify → template → store → <<<CCR:hash|type|size>>>
                                              │
                                              ▼
                                         Agent (honest preview, not raw output)
```

Four pipeline stages, all under 1ms:

1. **Classify** - 26-type regex classifier (`diff`, `build_output`, `code_rust`, …)
2. **Template** - TOML-driven preview templates per content type
3. **Store** - blake3 hash → SQLite or in-memory → CCR marker
4. **Preview** - Structured `[type:metadata]` the LLM reads instantly

---

## Architecture

```
src/
├── lib.rs               ← 25 C ABI exports for dylib loading
├── proxy.rs             ← HTTP proxy (:9797 cache, :9798 token)
├── hooks.rs             ← transform_tool_result, terminal, pre/post LLM
├── session.rs           ← Turn lifecycle, conversation index, catalog
├── state.rs             ← AphroditeState, inline store, LRU, markers
├── marker.rs            ← CCR marker formatting + parsing (<<<CCR:…>>>)
├── catalog.rs           ← Full/compact/TOC catalog display
├── chain_split.rs       ← Fine-grained chain splitting (SEG_MARKER segments)
├── resolve.rs           ← Recursive CCR expansion (5 levels deep)
├── stage2.rs            ← Semantic reduction for JSON, build, diff, code
├── struct_extract.rs    ← Code structure maps (Rust, Python, Go, JS/TS)
├── config.rs            ← CLI args + TOML multi-config
├── config_loader.rs     ← TOML + env var loading
├── directives.rs        ← Conversational directives: load, inject, list/swap/add/remove/reset
├── flow.rs              ← Per-turn injected-context assembler (directives + nudges + recall catalog)
├── setup.rs             ← `aphrodite setup` one-shot bootstrap (Gatekeeper-safe artifact install)
├── prefetch.rs          ← Background file read + compress threads
├── poll_worker.rs       ← Auto-backgrounding of slow tool calls
├── preview.rs           ← detect_type, build_preview (shared across proxy + dylib)
├── main.rs              ← Binary entry point
└── retrieve.rs          ← POST /retrieve - hash → content, filter, paginate, truncated flag
```

## C ABI (25 functions)

The 25 `#[no_mangle] extern "C"` exports in `lib.rs` (22 written out plus 3
macro-generated: `aphrodite_compress`, `aphrodite_transform`,
`aphrodite_terminal`) form the core C ABI. The `aphrodite-hermes` bridge
crate links this crate as an rlib and exposes its own higher-level ABI
(`libaphrodite_hermes.dylib` - see `crates/aphrodite-hermes/README.md`); the
Python plugin's ctypes bindings (`_bindings.py`) are generated from the bridge
header, not from these exports.

---

## Dependencies

- `headroom-core` - Content transforms + classifier + SQLite backend (vendored fork at `vendor/headroom/`)
- `axum` / `tokio` / `tower-http` - HTTP proxy (optional, gated behind `proxy` feature)
- `serde` / `serde_json` - Serialization
- `blake3` - Content-addressed hashing

---

## See Also

- [Installing Aphrodite](https://github.com/PlayForm/Aphrodite/tree/Development/docs/install/README.md) - which artifact you
  need, per-platform install guides, troubleshooting
- [aphrodite.toml Configuration](https://github.com/PlayForm/Aphrodite/tree/Development/docs/config/aphrodite-toml.md) - full
  TOML schema this crate's `config.rs` deserializes
- [Hermes Integration](https://github.com/PlayForm/Aphrodite/tree/Development/docs/hermes-integration.md) - how this binary's
  sibling dylib crate (`aphrodite-hermes`) plugs into Hermes Agent

## License

CC0-1.0 - public domain.
