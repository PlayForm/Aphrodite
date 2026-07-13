# aphrodite-hermes 🔌 Hermes Bridge

> **Hermes Agent integration layer - tool schemas, hook dispatch, skill registration.**

This crate is the bridge between the core `aphrodite` engine and the Hermes Agent
plugin system. It produces `libaphrodite_hermes.dylib` - loaded by the Python
plugin to register tools, hooks, and skills with Hermes.

[crates.io](https://crates.io/crates/aphrodite-hermes) ·
[docs](../../docs/README.md)

---

## Install

```bash
# From source (monorepo)
cargo build --release -p aphrodite-hermes

# From crates.io
cargo install aphrodite-hermes
```

---

## What it does

```
Hermes Agent
    │
    │  plugin load
    ▼
Python __init__.py (145 lines)
    │
    │  ctypes FFI
    ▼
libaphrodite_hermes.dylib  ← THIS CRATE
    │
    │  C ABI calls
    ▼
libaphrodite.dylib         ← Core engine
```

The bridge provides:

1. **Tool schemas** - 12 JSON Schema definitions for `aphrodite_*` tools
2. **Tool dispatch** - Routes Hermes tool calls to core engine functions
3. **Skill registration** - Bundled skills exposed to Hermes agents
4. **Hook dispatch** - Forwards hook calls (pre_llm, post_tool, etc.) to engine

---

## Architecture

```
src/
├── lib.rs          ← Universal dispatch: 14 hooks → Rust functions
├── tools.rs        ← 12 tool handler implementations
├── schemas.rs      ← JSON Schema for all tools
├── skills.rs       ← Bundled skill registration
```

---

## Tools

All 12 tools are registered as closures in one `tool_registry()` HashMap in
`src/tools.rs` (not separate per-tool modules) - full schemas and handler
behavior in [Tool Relay: Tools](../../docs/tool-relay/tools.md):

| Tool                        |
| :-------------------------- |
| `aphrodite_compress`        |
| `aphrodite_retrieve`        |
| `aphrodite_stats`           |
| `aphrodite_catalog`         |
| `aphrodite_search`          |
| `aphrodite_diff`            |
| `aphrodite_files`           |
| `aphrodite_test`            |
| `aphrodite_rebuild`         |
| `aphrodite_reclassify`      |
| `aphrodite_prefetch`        |
| `aphrodite_prefetch_status` |

`tool_registry()` also holds a 13th, internal-only entry -
`context_engine_pre_llm` - the context engine's pre-LLM hook, not a
Hermes-callable tool (it isn't in `plugin.yaml`'s `provides_tools`).

`aphrodite_diff` returns each turn's last-archived marker (`conv_index`),
populated by `hooks::post_llm_call` calling `session::archive_turn` at the
end of every turn (report 06 F11/T13 - previously `archive_turn` was dead
code with zero call sites, so this always returned `{"total": 0}`).

---

## Dependencies

- `aphrodite` - Core engine crate (path + version)
- `serde` / `serde_json` - JSON Schema + serialization

---

## See Also

- [Installing Aphrodite](../../docs/install/README.md) - which artifact you
  need, per-platform install guides, troubleshooting
- [Tool Relay: Tools](../../docs/tool-relay/tools.md) - full schema +
  handler behavior for all 12 tools this crate dispatches
- [Hermes Integration](../../docs/hermes-integration.md) - why a native
  plugin sees things a generic HTTP proxy can't

## License

CC0-1.0 - public domain.
