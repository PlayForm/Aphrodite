# Modular Plugin Splitting — Aphrodite Monolith Refactor

## How to split a monolithic Hermes plugin `__init__.py` into atomic modules

### 1. Read the full file first
```python
read_file(path="plugins/aphrodite/__init__.py")
```
Identify section boundaries via `# ── Section Name ──` comments and `def`/`class` declarations.

### 2. Map the sections to modules

| Section | Module | Contents |
|---|---|---|
| Constants, thresholds, shared state | `_core.py` | PORTS, REPO, BIN_VERSION, thresholds, `_CCR_RE`, `_inline_store`, shared mutable state |
| Inline compression | `_inline.py` | `_inline_compress()`, `_inline_retrieve()` |
| Marker formatting | `_marker.py` | `_ccr_marker()`, `_compress_via_proxy()`, `_parse_ccr_markers()` |
| Binary management | `_binary.py` | `_detect_platform()`, `_download_binary()`, `_ensure_binary()` |
| Proxy lifecycle | `_proxy.py` | `_load_env()`, `_alive()`, `_start()`, `on_start()`, `_wait_alive()` |
| CCR resolution | `_resolve.py` | `_resolve_one()`, `_resolve_recursive()` |
| Core tools | `_tools.py` | `_retrieve_handler()`, `_compress_handler()`, schemas |
| Hooks + remaining tools | `_hooks.py` | Transform hooks, rebuild, stats, files, diff, search, test, catalog |
| Context engine | `_engine.py` | `AphroditeContextEngine`, `get_engine()` |
| Public API | `__init__.py` | Re-exports, `register()`, debug banner |

### 3. Extract each section

Use `terminal` to extract line ranges, or `write_file` for each module. Each module gets:
- Module docstring
- Imports (only what it needs from `_core` and other modules)
- Its functions

### 4. Move shared state to `_core.py` FIRST

**Critical**: Any mutable state accessed by multiple modules MUST live in `_core.py`:
```python
# _core.py
_referenced_files = {}
_recent_markers = []
_conv_index = {}
_turn_counter = 0
_git_cache = {}
_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}

def _fmt_size(b): ...
def _inline_clear(): ...
```

All modules import from `_core`:
```python
from ._core import _recent_markers, _referenced_files, _fmt_size, ...
```

### 5. Remove duplicate definitions

After moving state to `_core.py`, remove the local definitions from `_hooks.py`, `_tools.py`, and `_resolve.py`. These become `F811 redefined-while-unused` errors.

### 6. Resolve circular imports

The most common failure: `_hooks.py` imports from `_engine.py`, and `_engine.py` imports from `_hooks.py`.

Fix: move any symbol imported by BOTH sides of the cycle into `_core.py`. In the aphrodite case, `_fmt_size`, `_inline_clear`, and all shared state needed to move from `_hooks` to `_core`.

### 7. Rewrite `__init__.py`

The new `__init__.py` is thin — just imports and the `register()` function:
```python
from ._core import PORTS, PLUGIN_VERSION, ...
from ._tools import COMPRESS_SCHEMA, RETRIEVE_SCHEMA, ...
from ._hooks import _transform_tool_result, _pre_llm_hook, ...
from ._engine import AphroditeContextEngine, get_engine

def register(ctx):
    # register hooks + tools + engine
```

### 8. Verify imports

```bash
cd /path/to/repo && python3 -c "import sys; sys.path.insert(0,'plugins'); import aphrodite; print('OK')"
```

### 9. Set up linting

Create `pyproject.toml` with ruff config. Add per-file-ignores for extracted code that uses the original file's style (inline try/except, `l` variable names).

### 10. Fix all lint errors

```bash
ruff check . --fix && ruff format .
ruff check . --statistics  # should show 0
```

### Pitfalls

- **Use `write_file` for the new `__init__.py` and each module** — patch is too fragile for full-file rewrites
- **Test imports after EVERY structural change** — don't batch 5 import changes and then test
- **`global` declarations in extracted functions**: If a function used `global _turn_counter` in the monolith, it must keep that declaration in the new module. Imported ints cannot be reassigned without `global`.
- **Duplicate `_engine = None`**: The extraction script may copy module-level state twice — check for duplicates
- **Don't use `terminal` to test repeatedly**: If imports fail 5 times, it hits the guardrail. Fix via `patch`/`write_file`, not repeated terminal calls.
