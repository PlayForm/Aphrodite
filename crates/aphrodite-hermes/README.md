# aphrodite-hermes 🔌 Hermes Bridge

> **Hermes Agent integration layer — tool schemas, hook dispatch, skill registration.**

This crate is the bridge between the core `aphrodite` engine and the Hermes Agent
plugin system. It produces `libaphrodite_hermes.dylib` — loaded by the Python
plugin to register tools, hooks, and skills with Hermes.

[crates.io](https://crates.io/crates/aphrodite-hermes) ·
[docs](https://github.com/PlayForm/Aphrodite)

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

1. **Tool schemas** — 12 JSON Schema definitions for `aphrodite_*` tools
2. **Tool dispatch** — Routes Hermes tool calls to core engine functions
3. **Skill registration** — Bundled skills exposed to Hermes agents
4. **Hook dispatch** — Forwards hook calls (pre_llm, post_tool, etc.) to engine

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

| Tool                   | Dispatch target      |
| :--------------------- | :------------------- |
| `aphrodite_compress`   | `hooks::compress`    |
| `aphrodite_retrieve`   | `hooks::retrieve`    |
| `aphrodite_stats`      | `hooks::stats`       |
| `aphrodite_catalog`    | `catalog::handler`   |
| `aphrodite_search`     | `search::handler`    |
| `aphrodite_diff`       | `diff::handler`      |
| `aphrodite_files`      | `files::handler`     |
| `aphrodite_test`       | `test::handler`      |
| `aphrodite_rebuild`    | `rebuild::handler`   |
| `aphrodite_reclassify` | `reclassify::handler`|
| `aphrodite_prefetch`   | `prefetch::handler`  |
| `context_engine_pre_llm`| `hooks::pre_llm`    |

---

## Dependencies

- `aphrodite` — Core engine crate (path + version)
- `serde` / `serde_json` — JSON Schema + serialization

---

## License

CC0-1.0 — public domain.
