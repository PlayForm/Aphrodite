<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋 (`aphrodite`)

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> **Your LLM burns 90% of its context on output it never reads. We fix that.**

[![release](https://img.shields.io/badge/release-v0.9.4-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v1.62.62-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## What You Get After Install

You clone, symlink, enable, and start Hermes. Here's exactly what happens:

```
$ hermes
💋 aphrodite v1.62.62  —  cache=UP token=UP
```

On first launch, Aphrodite auto-downloads its binary (~12MB). Then it launches
two local proxies — cache (:9797) and token (:9798) — and wires itself into
Hermes. From this point on, **every piece of output your agent sees is
compressed.**

### Your Agent Sees This

Instead of raw terminal dumps and file contents, your agent sees compact
structured previews:

| Tool output         | Agent sees (before)              | Agent sees (with Aphrodite)     |
|---------------------|----------------------------------|---------------------------------|
| `read_file main.rs` | 400 lines of Rust code           | `[code_rust:3fns 414L]`         |
| `cargo build`       | 200 lines of compile output      | `[build:1E 2W 142L]`            |
| `git diff`          | 500 lines of diff                | `[diff:5F +12/-8 340L]`         |
| `search_files`      | 50 results across 10 files       | `[search:50 results 5L]`        |
| Traceback           | 40 lines of Python traceback     | `[error:AttributeError 'None']` |

The agent reads 15 tokens of metadata instead of 500 tokens of raw text.
When it actually needs the full content, it calls `aphrodite_retrieve(hash)`.

### You Get These Tools

| Tool | What it does |
|------|-------------|
| `aphrodite_retrieve` | Fetch full content from a CCR marker |
| `aphrodite_compress` | Manually compress content into CCR |
| `aphrodite_stats` | Check proxy health, compression stats |
| `aphrodite_catalog` | Browse all compressed content |
| `aphrodite_search` | Search compressed content by keyword |
| `aphrodite_diff` | View conversation turn history |
| `aphrodite_files` | List all referenced files |
| `aphrodite_prefetch` | Load files in background |
| `aphrodite_rebuild` | Rebuild and restart the binary |
| `aphrodite_test` | Run smoke tests |

### Compression Is Automatic

You don't call anything. The plugin intercepts output at the hook level:

1. Agent runs `read_file` → Hermes gets 400 lines
2. Aphrodite hook fires → content classified as `code_rust`
3. Content compressed → `[code_rust:3fns 414L]` shown to agent
4. Agent continues reasoning with the preview
5. If agent needs details → calls `aphrodite_retrieve(hash)` → gets full content

---

## Install ⚡

```bash
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
ln -s "$(pwd)/Aphrodite-Hermes" ~/.hermes/plugins/aphrodite
hermes plugins enable aphrodite
hermes
```

That's it. No config needed. Set `APHRODITE_API_KEY` if your LLM provider requires one.

---

## Dev Workflow 🦀

Aphrodite's compression engine is Rust. The Python plugin is a thin loader.
You can iterate on the Rust code with zero-friction hot-reload:

```bash
# Terminal 1: watch for changes
APHRODITE_NO_AUTO_LAUNCH=1 cargo watch -x 'build -p aphrodite'

# Terminal 2: run Hermes (auto-loads rebuilt dylib)
hermes --profile dev-aphrodite
```

| Change | What happens |
|--------|-------------|
| Edit `.rs` file | cargo watch rebuilds dylib |
| Next hook call | Python detects mtime change → reloads dylib |
| Edit `.py` file | `/quit` + restart (Python import cache) |

---

## Architecture 🏗️

```
Python (thin loader ~150 lines)     Rust dylib (all logic ~5,000 lines)
  __init__.py                         libaphrodite.dylib
  headroom_ffi.py ← 18 C ABI funcs     ├── hooks.rs      (transform)
    ↓ ctypes FFI                       ├── resolve.rs    (resolution)
  libaphrodite.dylib                   ├── stage2.rs     (reduction)
                                       ├── state.rs      (inline store)
                                       ├── catalog.rs, session.rs, marker.rs
                                       └── lib.rs        (C ABI surface)
```

All 14 hooks and 12 tools delegate to the Rust dylib. The Python code serves
as fallback. Hot-reload works via mtime detection — rebuild the dylib and
the next hook call picks up the new code.

---

## vs Headroom

Aphrodite embeds [Headroom](https://github.com/PlayForm/Headroom) — our custom
fork. The difference: Headroom makes content smaller. Aphrodite makes it
optional.

| | Stock Headroom | Aphrodite |
|---|---|---|
| Agent sees | Smaller content | Preview (skip entirely) |
| Savings | 30-80% | 84%+ |
| Latency | ~100ms (ML) | <0.1ms (regex) |
| Hermes integration | None | Native plugin |
| Dependencies | Python + ML | Zero (Rust binary) |

---

_CC0‑1.0 — public domain. A PlayForm project._
