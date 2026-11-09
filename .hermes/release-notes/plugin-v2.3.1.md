## Aphrodite Hermes Plugin v2.3.1 💋

CCR compression plugin for Hermes Agent - thin Python loader over the Rust
dylib (all compression logic lives in the binary built by the parent
[PlayForm/Aphrodite](https://github.com/PlayForm/Aphrodite) release).

### What's included

- **Loader** (`__init__.py`): registers 6 hooks + 13 tools with Hermes via
  ctypes FFI - no compression logic in Python.
- **Binaries**: fetched from the parent's `Aphrodite/v1.8.0` GitHub Release via
  `download.sh` / `download.ps1` (pinned by `BINARY_VERSION` 1.8.0, SHA-256
  verified against the in-tree `SHA256SUMS.txt`).
- **Version**: plugin 2.3.1 ↔ binary 1.8.0.

### Changes since 2.3.0

- **A missing `aphrodite.toml` is now reported** instead of inferred. Without a
  config the binary falls back to a single token-mode listener on the first
  port - the second proxy never binds - and every `[compression]` knob,
  including the store retention caps below, is absent. `aphrodite setup` writes
  the file first, so the warning points at the real cause.
- **README**: the config sample carries the `[compression]` retention block.
- **`SHA256SUMS.txt`**: regenerated for the `Aphrodite/v1.8.0` binaries.
- **Windows**: `tests/test_windows_process_state.py` updated alongside the 1.8.0
  process-state handling.

### Store retention (new in the 1.8.0 binary)

`[compression] store_max_entries` (default `100000`) and `store_max_bytes`
(default `536870912`, payload bytes) evict least-recently-accessed rows past
either cap, batched at 256 rows per statement. `0` = unlimited. Env:
`APHRODITE_STORE_MAX_ENTRIES` / `APHRODITE_STORE_MAX_BYTES`. Caps apply when the
store opens - an already-oversized DB is trimmed at open - and are not
hot-reloaded. An evicted hash degrades to a retrieval miss, exactly like an
expired one. See
[#53](https://github.com/PlayForm/Aphrodite/issues/53).

### Features (shipped via the Rust engine)

TOML-driven templates, auto-expand, default-on context engine (middle-turn
archiving + engine_offload), live containers, prompt knobs, recursion/body
caps, optional Rhai scripting - full detail in the parent release notes:
https://github.com/PlayForm/Aphrodite/releases/tag/Aphrodite/v1.8.0

### Repository layout

This repository is the plugin submodule of the parent monorepo. See the
[Repositories](https://github.com/PlayForm/Aphrodite-Hermes#repositories)
section of the README for the two-repo structure and the release ceremony.
