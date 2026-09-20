# aphrodite-hermes 🔌 Hermes Bridge

> **Hermes Agent integration layer - tool schemas, hook dispatch, skill registration.**

This crate is the bridge between the core `aphrodite` engine and the Hermes Agent
plugin system. It produces `libaphrodite_hermes.dylib` - loaded by the Python
plugin to register tools and hooks with Hermes (bundled skills ship via the
plugin directory, not this crate).

[crates.io](https://crates.io/crates/aphrodite-hermes) ·
[docs](https://github.com/PlayForm/Aphrodite/tree/Development/docs/README.md)

---

## Install

`cargo install aphrodite-hermes` only ships the setup-helper binary - the
dylib the plugin needs is a library-crate output that `cargo install` never
distributes (see `src/bin/setup-helper.rs`). The working clone-and-setup path:

```bash
# 1. Build both crates from a checkout (binary + bridge dylib)
cargo install --path crates/aphrodite
cargo build --release -p aphrodite-hermes

# 2. Bootstrap the runtime home (~/.hermes/aphrodite/: binaries/, directives/,
#    hotreload/, ccr.db) - copies the binary and dylibs into binaries/
aphrodite setup

# 3. Link the plugin into Hermes's plugin directory (manual step)
ln -s "$(pwd)/plugins/aphrodite" ~/.hermes/plugins/aphrodite
```

After source changes, rebuild and re-run `aphrodite setup` so `binaries/`
picks up the new dylib; the plugin hot-reloads it on mtime change.

---

## What it does

```
Hermes Agent
    │
    │  plugin load
    ▼
Python __init__.py (thin loader)
    │
    │  ctypes FFI
    ▼
libaphrodite_hermes.dylib  ← THIS CRATE
    │
    │  links core as rlib
    ▼
aphrodite crate (rlib)     ← Core engine (no separate dylib load)
```

The bridge provides:

1. **Tool schemas** - 13 JSON Schema definitions for `aphrodite_*` tools
2. **Tool dispatch** - Routes Hermes tool calls to core engine functions
3. **Directive provisioning** - Materializes the embedded builtin directives
   into the runtime home (`~/.hermes/aphrodite/directives/`)
4. **Hook dispatch** - Forwards hook calls (on_session_start, pre_tool_call,
   transform_tool_result, transform_terminal_output, pre_llm_call,
   post_llm_call) to engine

---

## Architecture

```
src/
├── lib.rs          ← Universal dispatch: 6 hooks → Rust functions
├── tools.rs        ← 13 tool handler implementations
├── schemas.rs      ← JSON Schema for all tools
├── directives.rs   ← Builtin directive materialization into the runtime home
├── bin/setup-helper.rs ← cargo-install helper (the only artifact `cargo install` ships)
└── build.rs        ← FFI codegen: cbindgen → header → ctypesgen → _bindings.py
```

---

## Tools

All 13 tools are registered as closures in one `tool_registry()` HashMap in
`src/tools.rs` (not separate per-tool modules) - full schemas and handler
behavior in [Tool Relay: Tools](https://github.com/PlayForm/Aphrodite/tree/Development/docs/tool-relay/tools.md):

| Tool                        |
| :-------------------------- |
| `aphrodite_compress`        |
| `aphrodite_retrieve`        |
| `aphrodite_stats`           |
| `aphrodite_catalog`         |
| `aphrodite_search`          |
| `aphrodite_diff`            |
| `aphrodite_files`           |
| `aphrodite_directive`       |
| `aphrodite_test`            |
| `aphrodite_rebuild`         |
| `aphrodite_reclassify`      |
| `aphrodite_prefetch`        |
| `aphrodite_prefetch_status` |

`tool_registry()` also holds a 14th, internal-only entry -
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
- `cbindgen` (build-dependency) + `ctypesgen` (external tool) - FFI codegen for `_bindings.py`

---

## See Also

- [Installing Aphrodite](https://github.com/PlayForm/Aphrodite/tree/Development/docs/install/README.md) - which artifact you
  need, per-platform install guides, troubleshooting
- [Tool Relay: Tools](https://github.com/PlayForm/Aphrodite/tree/Development/docs/tool-relay/tools.md) - full schema +
  handler behavior for all 13 tools this crate dispatches
- [Hermes Integration](https://github.com/PlayForm/Aphrodite/tree/Development/docs/hermes-integration.md) - why a native
  plugin sees things a generic HTTP proxy can't

## License

CC0-1.0 - public domain.
