---
name: aphrodite-dev-workflow
description: "End-to-end aphrodite plugin + proxy development: cargo watch, WezTerm MCP, Hermes testing, binary build, git workflow."
version: 1.7.0
platforms: [macos]
related_skills: [aphrodite-hook-reference, aphrodite-tool-guide]
---

# Aphrodite Development Workflow

Complete development cycle for the aphrodite Hermes plugin and Rust proxy binary.

## When to Load

Any aphrodite development: plugin code changes, proxy Rust changes, hook debugging, testing via Hermes, binary rebuilds, git workflow. Also load `aphrodite-tool-guide` for the user-facing tool reference (CCR lifecycle, catalog vs search, stats interpretation).

**CRITICAL - agent must load this skill at session start**: The system prompt lists aphrodite skills but the agent does not auto-load them. The agent will happily run benchmarks, read engine code, and report metrics without ever loading this skill - missing key pitfalls like the cache-vs-token profile issue, the `--no-optimize` headroom passthrough, and the mutual exclusion of `compression.enabled` and `context.engine`. If the agent hasn't loaded this skill after the first user message about aphrodite, load it immediately.

## Architecture

```
HermesCompress/
├── plugins/aphrodite/__init__.py    # Python plugin (hooks + tools)
├── plugins/aphrodite/plugin.yaml    # Plugin manifest
├── crates/aphrodite/src/            # Rust proxy binary
├── aphrodite.toml                   # Multi-proxy config
└── Cargo.toml                       # Workspace root
```

Plugin registers 5 hooks + 7 tools. Rust proxy runs two listeners:
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold

## Dev Environment Setup

### Plugin Reload - NO HOT RELOAD

Hermes caches plugin code in memory at session start. Full cycle required: `hermes plugins disable aphrodite && hermes plugins enable aphrodite`, then RESTART Hermes. Even after `/quit` + restart, Python's `import` system may cache the old module in `sys.modules`. Delete `__pycache__` to force re-import.

### Context Compression - Common Misconfiguration

If Hermes compacts context on every turn: `ENGINE_THRESHOLD_PCT=0` (fixed in v0.5.16: 0=disabled, 50=normal) OR Hermes' built-in `compression.enabled: true`. Fix: `hermes config set compression.enabled false`.

### Debug Banner

`APHRODITE_DEBUG=1` shows debug via `print()` (TUI) + `_log.info()` (agent.log). v0.5.42 adds `⚙` debug lines in `[APHRODITE]` catalog.

### Port Squatting

`lsof -ti:9797 -ti:9798 | xargs kill -9` + `pkill -9 -f cargo-watch`. Verify: `curl -s http://127.0.0.1:9798/health || echo free`. Panic at proxy.rs:762 fixed in v0.5.45 (saturating_sub).

### Full Dev Environment Setup

### Full Setup Sequence (WezTerm MCP)

Pane layout: pane 9 = proxy (cargo watch), pane 8 = Hermes test.

**Start proxy (pane 9) - filtered logging:**
```
mcp_wezterm_send_text(pane_id=9, text="pkill -9 -f cargo-watch; pkill -9 -f target/debug/aphrodite\n")
sleep 2
mcp_wezterm_send_text(pane_id=9, text="APHRODITE_API_KEY=*** APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'\n")
# RUST_LOG=aphrodite=trace,info shows aphrodite-level trace but hides hyper_util/rustls/reqwest noise
# Wait for "listening addr=127.0.0.1:9798" in buffer before starting Hermes
```

**Start Hermes test (pane 8) - full plugin logging:**
```
mcp_wezterm_send_text(pane_id=8, text="/quit\n")  # if Hermes is running
sleep 2
mcp_wezterm_send_text(pane_id=8, text="APHRODITE_DEBUG=1 hermes --provider custom:aphrodite-token\n")
```

**Reset both panes after each edit/release:**
```
# Pane 9: double Ctrl+C for cargo watch, then kill
mcp_wezterm_send_text(pane_id=9, text="\x03\x03")
sleep 1
mcp_wezterm_send_text(pane_id=9, text="pkill -9 -f 'cargo-watch|target/debug/aphrodite'\n")
# Pane 8: quit Hermes
mcp_wezterm_send_text(pane_id=8, text="/quit\n")
sleep 2
# Verify both show clean shell prompt at HermesCompress dir, then restart
```

### Required Config for Hooks-Only Mode

Aphrodite runs best in hooks-only mode (no context engine, no Hermes built-in compression):

```bash
hermes config set compression.enabled false    # CRITICAL: disable Hermes built-in
hermes config set context.engine default       # skip aphrodite context engine
hermes plugins enable aphrodite                # hooks + tools still active
```

With this config, the plugin provides:
- 7 tools (retrieve, compress, stats, rebuild, files, diff, search)
- Hook compression (tool results, terminal output, build collapse)
- File tracking + tree injection in pre_llm_hook catalog
- All proxy features (content detection, adaptive thresholds, auto-tune)

Context engine is opt-in: set `APHRODITE_CONTEXT_ENGINE=1` env var + `context.engine: aphrodite` in config.

### Config: No Session Resume
```bash
hermes config set agent.resume_session false
hermes config set agent.tui_auto_resume_recent false
hermes config set display.tui_auto_resume_recent false
```
Multiple config sections may need the same key - check all with `grep`.

### WezTerm MCP Commands

| Action | Tool |
|--------|------|
| List panes | mcp_wezterm_list_panes() |
| Read buffer | mcp_wezterm_get_buffer(pane_id=N, lines=50) |
| Send command | mcp_wezterm_send_text(pane_id=N, text="cmd\\n") |

**CRITICAL - Verify after every action**: After every `mcp_wezterm_send_text`, call `mcp_wezterm_get_buffer(pane_id, lines=5)` to confirm the command landed. This is how a human works - type, look, verify. Never assume text was received.

**CRITICAL - Never paste onto running process**: If cargo watch or Hermes is running, sending text pastes into the process's stdin, not the shell. Always `pkill -9` first, verify clean shell with `get_buffer`, then send. NO exceptions.

**CRITICAL - Never use mcp_wezterm_send_text with \x03**: WezTerm send-text does NOT send raw Ctrl+C bytes - it pastes the literal text \`\x03\`. Always use \`terminal(command="pkill -9 -f cargo-watch; pkill -9 -f target/.*aphrodite", timeout=5)\` instead. NEVER use mcp_wezterm_send_text for process control.

## Compression Engine Details

### Content Type Detection (v0.5.10+)

`detect_content_type()` in `proxy.rs` identifies 13 content categories via first-line + pattern matching:

| Category | Detection Pattern | Threshold Multiplier |
|----------|-------------------|---------------------|
| `error` | `Traceback`, `panic`, `thread '`, first-line contains `error`/`Error`/`ERROR` | ×8 (always visible) |
| `code_rust` | `fn ` + (`-> `, `impl `, `struct `, `pub `) | ×4 |
| `code_python` | `def ` + (`import `, `class `, `from `, `self.`) | ×4 |
| `code_go` | (`func ` or `package `) + `import (` | ×4 |
| `code_js` | (`function ` or `const ` or `=> `) + (`import ` or `export `) | ×4 |
| `code` | generic: `fn `, `def `, `class `, `import `, `pub fn` | ×4 |
| `diff` | first line starts with `diff --git`, `@@ -`, `+++`, `---` | ×2 |
| `git` | first line starts with `commit `, `On branch ` | ×2 |
| `build_output` | first line starts with `Compiling`, `Finished`, `running`, `test ` | ÷2 (aggressive) |
| `log` | >5 lines, no code/error patterns | ÷2 (aggressive) |
| `tool_output` | JSON with `exit_code` or `"status"` fields | ×1 (default) |
| `json` | starts with `{` or `[` | ×1 |
| `text` | fallback | ×1 |

### Adaptive Thresholds

`AppState::threshold_for(ct)` applies per-type multipliers to the base threshold:

```
error:       base × 8   → errors stay visible even under token proxy (8KB floor)
code_*:      base × 4   → code files preserved (4KB token, 32KB cache)
diff/git:    base × 2   → moderately compressed
tool/json:   base × 1   → standard
build/log:   base ÷ 2   → aggressively compressed (512B token, 4KB cache)
```

The `compress_chat_completion` function now detects content type before the threshold check, using `state.threshold_for(ct).max(base_threshold)` so the result is never below the base threshold.

### Secret API Key Wrapper (v0.5.8+)

`api_key` in `AppState` is wrapped in a `Secret(String)` newtype with safe `Debug` (prints `[REDACTED]`) and `Display` (passes through for HTTP headers). Prevents accidental logging. Add to structs:

```rust
pub struct Secret(pub(crate) String);
impl Debug for Secret { fn fmt(...) -> write!(f, "[REDACTED]") }
impl Display for Secret { fn fmt(...) -> write!(f, "{}", self.0) }
```

### Header Forwarding (v0.5.8+)

All `x-headroom-*` headers (session-id, trace-id, etc.) now pass through to upstream. Previously only `x-headroom-workspace` was forwarded; others were silently dropped. The skip block was removed.

### Port Squatting Fix

When cargo watch restarts, old proxy processes may still hold ports 9797-9798. The sequence:

```bash
# Kill everything holding the ports (terminal tool, not MCP)
pkill -9 -f "cargo-watch" 2>/dev/null
pkill -9 -f "target/.*aphrodite" 2>/dev/null
lsof -ti:9797 -ti:9798 2>/dev/null | xargs kill -9 2>/dev/null
sleep 1
# Verify ports freed
curl -s http://127.0.0.1:9798/health 2>/dev/null || echo "free"
```

The `lsof` approach kills processes by port, catching orphaned aphrodite processes that `pkill` misses.

**NEVER use `\x03` (Ctrl+C) via MCP** - it doesn't kill cargo watch reliably. Always use `pkill -9`.

### Plugin Code Reload

Hermes caches plugin code at session start. Changes require a full Hermes restart. The `hermes plugins disable/enable` only flips the enabled flag - it does NOT reload code.

To force a clean reload:
```bash
hermes plugins disable aphrodite && hermes plugins enable aphrodite
find ~/.hermes/plugins/aphrodite -name "*.pyc" -delete
find ~/.hermes/plugins/aphrodite -name "__pycache__" -type d -exec rm -rf {} +
/quit  # in Hermes pane
APHRODITE_DEBUG=1 hermes --provider custom:aphrodite-token  # restart
```

Verify with: `grep "v1\.[0-9][0-9]\.[0-9]" ~/.hermes/logs/agent.log | tail -1`

### Diagnosing Proxy Auth Issues

1. Check proxy health: `curl http://127.0.0.1:9798/health`
2. Logs show 401 if API key missing
3. aphrodite.toml needs api_key in [defaults] or env var passed
4. **Aphrodite Rust proxy** reads `APHRODITE_API_KEY` env (Key A)
5. **Headroom Python proxy** uses litellm, reads `HEADROOM_DEEPSEEK_KEY` or `DEEPSEEK_API_KEY` (Key B)
6. cargo watch may not inherit env - pass inline: `APHRODITE_API_KEY=... cargo watch ...`
7. Headroom proxy needs key in env before launch: `source ~/.privateenvsh && headroom proxy ...`

## Build + Deploy

### Rebuild Binary (after Rust changes)

Use aphrodite_rebuild() tool, or:
```
cargo build --release -p aphrodite
cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite
chmod 755 ~/.hermes/aphrodite/aphrodite
```

### Plugin Changes: Reload Requirements

| Change | Required |
|--------|----------|
| Hook function bodies | Hermes restart |
| register() changes | Hermes restart |
| Config values | 5s cache, auto |
| Rust binary | Next session_start |

### Context Engine Activation (OPT-IN)

The context engine is opt-in. Aphrodite runs best in hooks-only mode.

**WARNING - mutual exclusion**: `compression.enabled` and `context.engine: aphrodite`
are TWO DIFFERENT compression systems and MUST NOT both be active at the same time:

| Config | What it does | When active |
|--------|-------------|-------------|
| `compression.enabled: true` | Hermes' built-in summarizer - generic truncation, no CCR markers, no content-type awareness | Shows "🗜️ Compacting context" in TUI |
| `context.engine: aphrodite` | Our engine - CCR-based, content-type adaptive (errors ×8, logs ÷2), tool-chain safe | Offloads middle messages to proxy, keeps head/tail raw |

**Both ON = bad**: They fire independently and fight each other. Hermes compresses →
aphrodite sees already-compressed messages → tries to compress again → degrades quality
and can loop. The ONLY valid configs are:

```bash
# Option A: hooks-only (recommended default)
hermes config set context.engine default
hermes config set compression.enabled false

# Option B: context engine (opt-in, never with compression.enabled: true)
hermes config set context.engine aphrodite
hermes config set compression.enabled false   # MUST be false
```

**To enable context engine:**
```bash
hermes config set context.engine aphrodite
hermes config set compression.enabled false   # CRITICAL
```

The engine activates on next session start (not mid-session). It triggers based on
env-configurable thresholds. Defaults:
- `APHRODITE_ENGINE_THRESHOLD_PCT=50` - compress at 50% context fill
- `APHRODITE_ENGINE_MIN_MSGS=30` - don't compress short conversations
- `APHRODITE_ENGINE_PROTECT_FIRST=2` / `PROTECT_LAST=5` - keep head/tail raw

Set `APHRODITE_ENGINE_THRESHOLD_PCT=0` to disable compression entirely.
The `should_compress()` method returns False when: threshold_pct is 0, prompt_tokens is 0, or context_length is missing.

### Context Compression Troubleshooting

If Hermes shows "🗜️ Compacting context" on every turn:

1. **Root cause**: Hermes' built-in `compression.enabled: true` triggers independently of aphrodite. Even with `context.engine: aphrodite`, Hermes runs its own compressor.
2. **Fix**: `hermes config set compression.enabled false`
3. If using context engine, verify `APHRODITE_ENGINE_THRESHOLD_PCT` is not 0 (0 = disabled now, old default was 0 = always)
4. Verify `APHRODITE_ENGINE_MIN_MSGS` is reasonable (default 30)
5. Plugin enable/disable cycle doesn't affect running sessions - must restart CLI

## Git Workflow

Stage specific files only (never -A):
```
git add plugins/aphrodite/__init__.py plugins/aphrodite/plugin.yaml
git commit -m "type(aphrodite): description"
git push
```

Branch: Current tracking aphrodite/Current.

## Bi-Directional Store Pattern

Every compress and retrieve operation feeds the search index:

```
compress_handler → proxy → _inline_store[h] = content → searchable
transform_tool_result → proxy → _inline_store[h] + _recent_markers → searchable
_resolve_one → proxy → _inline_store[h] + _recent_markers → searchable
_search_handler → scans _inline_store + _conv_index + _recent_markers
```

Content-addressable: `_compress_handler` computes hash locally first. If hash exists in `_inline_store`, returns cache hit - no proxy call. This is the "pop the API" pattern: every put is a search.

## CCR Catalog Display Bugs

Three layers of defense against `CCR:{}` ghost entries:

1. **Parse level**: `_parse_ccr_markers` uses `re.finditer()` with `match.end()` for correct position; casts all hashes to `str()`; filters entries with `len(hash) < 4`
2. **Preview level**: `_extract_preview` splits on `>>>` (not `]` from old bracket format) to get actual content preview, not `|tool|token>>>` fragments
3. **Display level**: Catalog rendering skips hashes matching `''`, `{}`, `?`, `None`, `null`, `undefined`

See `references/ccr-empty-hash-bug.md` for the full two-layer fix pattern and root cause analysis.

## Task Assessment

`.hermes/tasks/` holds numbered audit files (1.md, 2.md, 3.md, 4.md) with systematic bug audits ranked by severity. `.hermes/plans/` holds architectural plans. Load task files in order when executing fixes - read all before making changes. See `references/bug-audit-execution.md` for the full priority-ordered workflow.

**PITFALL - wrong directory**: The task files live in `.hermes/tasks/`, NOT `.hermes/plans/`. The plans directory has architectural documents. When the user says "read from tasks", they mean `.hermes/tasks/1.md` etc., not `.hermes/plans/headroom-coding-rewrite.md`. Always confirm the directory before searching. When the user pastes a task list inline and says "for:", they're showing you what's in the task file - go read the actual files from `.hermes/tasks/`.

### `.hermes/` Organization

After completing a wave of bug fixes, reorganize `.hermes/` for clarity:
- Create `MASTER-TASKS.md` - comprehensive table of ALL bugs with status (✅/❌/⚠️), severity (🔴/🟠/🟡/🟢), file, version fixed
- Rename task files with descriptive numbered prefixes: `1-wave-audit.md`, `2-python-bugs.md`, etc.
- Rename plan files with numbered prefixes: `0-headroom-100-tasks.md`, `1-honest-gaps.md`, `2-architectural-subtasks.md`
- Remove duplicates (v2 copies, consolidated audits)
- Remove stale root files that are now consolidated into MASTER-TASKS.md
- Update `HANDOFF.md` and `AGENTS.md` to reflect current state
- Commit with `git add --force .hermes/` (directory is gitignored)

## Plugin Code Organization

The plugin's `__init__.py` must be kept modular - NOT a single monolithic file. Split into atomic modules:

```
plugins/aphrodite/
├── __init__.py     # orchestrator: imports, register(), debug banner
├── _core.py        # constants, thresholds, CCR regex, inline store, SHARED STATE
├── _inline.py      # zlib fallback compression (_inline_compress, _inline_retrieve)
├── _marker.py      # CCR marker formatting + proxy compression + _parse_ccr_markers
├── _binary.py      # platform detection, binary download, _ensure_binary
├── _proxy.py       # env loading, _alive(), _start(), on_start(), _wait_alive()
├── _resolve.py     # _resolve_one(), _resolve_recursive()
├── _tools.py       # core tool handlers (retrieve, compress + schemas)
├── _hooks.py       # hook handlers + remaining tools (rebuild, stats, files, diff, etc.)
├── _engine.py      # AphroditeContextEngine
├── pyproject.toml  # ruff + pyright config
├── pyrightconfig.json
└── plugin.yaml
```

Each module uses relative imports (`from ._core import ...`). `__init__.py` serves as the public API - imports from all submodules and re-exports everything. The `register()` function stays in `__init__.py` since it wires all modules together.

### Shared State Pattern - _core.py as Hub

Session-scoped mutable state that multiple modules need to access MUST live in `_core.py`, NOT in individual modules. This prevents circular imports:

```python
# _core.py - declare shared state once
_referenced_files = {}  # {filepath: last_tool_name}
_recent_markers = []     # [{hash, type, size, preview}]
_conv_index = {}         # {turn_num: (hash, summary, size)}
_turn_counter = 0
_git_cache = {}          # {ts, summary}
_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}

# Also: shared utility functions
def _fmt_size(b): ...   # byte formatting
def _inline_clear(): ... # clear inline store
```

Modules import from `_core`:
```python
# _hooks.py and _engine.py both import from _core
from ._core import _recent_markers, _referenced_files, _conv_index, _fmt_size, ...
```

### Circular Import Resolution

When splitting a monolithic file, circular imports are the most common failure pattern. The fix is to move any symbol imported by BOTH sides of the cycle into `_core.py`. Example: `_hooks.py` needs `get_engine()` from `_engine.py`, and `_engine.py` needs `_fmt_size`/`_recent_markers` from `_hooks.py` → move `_fmt_size` and shared state to `_core.py`, break the cycle. Always verify with `python3 -c "import aphrodite"` after restructuring imports.

### Python Linting Setup

Use ruff + pyright for the plugin. Config in `pyproject.toml`:

```toml
[tool.ruff]
target-version = "py311"
line-length = 120

[tool.ruff.lint]
select = ["E", "W", "F", "I", "N", "UP", "B", "C4", "SIM", "TCH"]
ignore = ["E501", "B008"]

[tool.ruff.lint.per-file-ignores]
"__init__.py" = ["F401"]  # re-exports intentional
"_hooks.py" = ["E701", "E741", "N806", "B007"]  # extracted code style
"_engine.py" = ["E701"]

[tool.ruff.format]
quote-style = "double"
indent-style = "space"
```

Install: `pip3 install uv ruff`. Run: `ruff check . --fix && ruff format .`. Use per-file-ignores for extracted code that intentionally uses inline try/except or `l` in list comprehensions - those are in the original file's style, not bugs.

### Pyright / Pylance Type Checking

Pylance in VS Code surfaces `reportUnknownArgumentType` and similar errors. The in-repo
`pyrightconfig.json` should use `strict` mode with all warnings enabled - this catches
missing type annotations that VS Code flags:

```json
{
    "include": ["."],
    "typeCheckingMode": "strict",
    "reportMissingImports": "warning",
    "reportMissingTypeStubs": "warning",
    "reportUnknownArgumentType": "warning",
    "reportUnknownMemberType": "warning",
    "reportUnknownVariableType": "warning",
    "reportUnknownParameterType": "warning",
    "reportMissingParameterType": "warning",
    "reportMissingTypeArgument": "warning",
    "reportGeneralTypeIssues": "warning",
    "reportOptionalMemberAccess": "warning"
}
```

**Background scan**: Launch as a hermes -z flash worker (never block waiting):

```bash
cd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress && \
hermes -z "Run strict pyright on plugins/aphrodite/. Write a temp pyrightconfig-full.json, run pyright --project /tmp/pyrightconfig-full.json ., report errors/warnings/file count." --model deepseek-v4-flash
```

Launch via `terminal(background=true)`. Results arrive via `process(action="poll")`.

## Testing the Context Engine

The context engine activates on session start when `context.engine: aphrodite` is in config.
Compression thresholds are env-configurable. See `references/context-engine-activation.md` for full gating mechanics, message-count dependency, and the 0-compressions debugging checklist.
See `references/verification-checklist.md` for the full post-change checklist.

### Verify engine loaded
Check Hermes startup logs for: `Using context engine: aphrodite`
If you see `Context engine 'aphrodite' not found - falling back to built-in compressor`,
the plugin didn't register the engine (check plugin.yaml has `provides_context_engine: true`).

### Verify compression works
1. Start a test Hermes session with `--provider custom:aphrodite-token`
2. Build context (10+ messages including tool calls)
3. Check proxy logs for: `context_engine: compressed N msgs → CCR:hash`
4. The message count should drop as middle messages are offloaded to CCR

### Troubleshooting
- Proxy must be alive for compression (`_alive()` returns True)
- Inline fallback uses zlib when proxy is down
- Context engine only works when `compression.enabled: false` (Hermes built-in must be OFF - see mutual exclusion above)

## Atomic Testing & Capability Comparison

The `examples/` directory has 16 self-contained Python regression tests and 4 Rust benchmarks. See `references/atomic-examples.md` for the full catalog, the 'buggy vs fixed' pattern, the normal-vs-CCR capability comparison workflow, and a note about CCR-marker-returned pipeline results.

Quick smoke: `python3 examples/01_env_var_typo.py && python3 examples/16_integration_smoke.py`

## Debugging

See `references/env-vars.md` for full threshold configuration.
See `references/debug-banner-output.md` for the full debug startup banner format and location.

### Enable Debug Logging
`APHRODITE_DEBUG=1 hermes ...`

**Where the output goes**: The debug banner and all plugin log messages go to `~/.hermes/logs/agent.log` via Python's `logging.info()`. They do NOT appear in the Hermes TUI. After starting a session with `APHRODITE_DEBUG=1`, check the log:
```bash
grep "APHRODITE.*DEBUG MODE" ~/.hermes/logs/agent.log | tail -1
grep -A10 "APHRODITE.*DEBUG MODE" ~/.hermes/logs/agent.log | tail -12
```

### Hook Not Firing
1. Hook name matches VALID_HOOKS (session_start -> on_session_start)
2. plugin.yaml provides_hooks lists correct name
3. APHRODITE_PASSTHROUGH not set (disables all hooks)
4. Proxy alive (_alive returns True)

### Output Being Eaten
1. Verify hook param names match Hermes invocation
2. Check for undefined 'result' variable returns
3. Hook string returns REPLACE output; None passes through

## Content Type Detection

The proxy's `detect_content_type()` identifies 13 content categories for adaptive compression thresholds. See `references/content-types.md` in the aphrodite skill references for the full taxonomy.

Key facts for debugging:
- Code (×4 threshold) preserves more content in context
- Error (×8) is never compressed unless extremely large
- Build output (÷2) and logs (÷2) compress aggressively
- The terminal hook has a separate build-collapse path for cargo/go/npm output

## Quiet Mode - Zero-Noise Operating Preference (Overridden in Dev)

For production: no telemetry, no tool-call echoes, no cost tracking, no debug spam. For development: verbose mode with `APHRODITE_DEBUG=1`, `display.compact: false`, `agent.show_tool_calls: true`, `logging.level: DEBUG`. Choose per session.

```bash
# Production (quiet):
hermes config set agent.show_tool_calls false
hermes config set display.compact true
hermes config set logging.level error
```

## Headroom Proxy Auth Chain

The headroom proxy uses **litellm** under the hood, which maps providers to env vars. For `api.deepseek.com`, litellm needs `DEEPSEEK_API_KEY` or `HEADROOM_DEEPSEEK_KEY` - NOT `OPENAI_API_KEY`.

1. **Two-key system**: Aphrodite/Hermes uses `APHRODITE_API_KEY` (Key A, sk-135 prefix). Headroom proxy uses `HEADROOM_DEEPSEEK_KEY` (Key B, sk-419 prefix) for upstream auth.
2. **Proxy launch**: Must have `DEEPSEEK_API_KEY` or `HEADROOM_DEEPSEEK_KEY` in its environment. Always `source ~/.privateenvsh` before launching: `source ~/.privateenvsh && headroom proxy --port 9799 ...`
3. **Hermes provider config**: Use `provider: deepseek` with `api_key_env: HEADROOM_DEEPSEEK_KEY`:
   ```bash
   hermes config set providers.headroom-cache.provider deepseek
   hermes config set providers.headroom-cache.api_key_env HEADROOM_DEEPSEEK_KEY
   hermes config set providers.headroom-cache.base_url http://127.0.0.1:9799
   ```
4. **WezTerm launch**: Proxy processes launched via WezTerm inherit the shell's env. Always source before launching.
5. **Upstream routing**: `--openai-api-url https://api.deepseek.com/v1` tells headroom where to forward.

Full debugging trace for 401 auth errors:
- Test both keys directly: `python3 -c "import urllib.request...; req = ... Authorization: Bearer $KEY"` - if 401, the key is expired
- Check proxy env: `ps aux | grep headroom` won't show env, test via `curl -s http://127.0.0.1:9799/v1/chat/completions` (200 = proxy has valid key)
- Check Hermes config: `hermes config show | grep headroom` - must show `api_key_env: HEADROOM_DEEPSEEK_KEY`
- If env shows wrong key prefix (`sk-71cf0` instead of `sk-419`), update `~/.privateenvsh` and `~/.hermes/.env`

### Profile Symlinking (Not Copying)

Profiles live in repo at `profiles/` and are symlinked from `~/.hermes/profiles/<name>/`. Plugin and skills are also symlinks - NOT copies and NOT text files containing paths:

**Canonical chain**: `~/.hermes/profiles/<name>/plugins/aphrodite → ~/.hermes/plugins/aphrodite → /path/to/HermesCompress/plugins/aphrodite`. One source, all profiles:

```bash
# Create the hub symlink (pointing to source)
ln -s /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress/plugins/aphrodite \
  ~/.hermes/plugins/aphrodite

# Each profile points to the hub
for prof in aphrodite-*; do
  rm -rf ~/.hermes/profiles/$prof/plugins/aphrodite
  ln -s ~/.hermes/plugins/aphrodite ~/.hermes/profiles/$prof/plugins/aphrodite
  rm -f ~/.hermes/profiles/$prof/skills
  ln -s ~/.hermes/skills ~/.hermes/profiles/$prof/skills
done
```

```bash
# FIX broken profiles (text file instead of symlink):
for prof in aphrodite-*; do
  test -f ~/.hermes/profiles/$prof/plugins/aphrodite && \
    rm ~/.hermes/profiles/$prof/plugins/aphrodite && \
    ln -s /path/to/repo/plugins/aphrodite ~/.hermes/profiles/$prof/plugins/aphrodite
  test -f ~/.hermes/profiles/$prof/skills && \
    rm ~/.hermes/profiles/$prof/skills && \
    ln -s ~/.hermes/skills ~/.hermes/profiles/$prof/skills
done
```

One source of truth - update the plugin once, all profiles see it instantly. Regenerate SOUL.md with `scripts/generate-soul.py`. Never `cp -r` plugin files into profiles.

### .env.sh for Env Management

Create `.env.sh` files with `export KEY=value` format next to each `.env`:

```bash
sed 's/^/export /' .env > .env.sh
```

Source project `.env.sh` to launch proxy/cargo watch. Never source `~/.privateenvsh`.

### Per-Profile Plugin Enable

Plugin must be enabled per-profile: `hermes plugins enable aphrodite --profile <name>`. Default is `not enabled`.

### Model Catalog for Custom Providers

Custom providers don't serve `/v1/models`. Populate config:
```yaml
model_catalog:
  providers:
    aphrodite-token: [deepseek-v4-pro, deepseek-v4-flash]
    aphrodite-cache: [deepseek-v4-pro, deepseek-v4-flash]
```

## Pitfalls & User Preferences

### User Communication
- Prefer direct action over explanation - fix things, don't describe them.
- "More" means continue iterating and improving - never stop to summarize.
- "More" and "deeply fine tune" signal: apply coding-specific optimizations, architectural changes welcome if fully developed.
- **"more?" after metrics is a depth-first escalation**: Each "more?" advances to the next data source layer (stats → proxy raw → process tree → config → git → lifetime savings → audit). Go deeper, not wider. Never repeat the same layer. When layer 6 (bug audit) is reached, say so explicitly. See `references/comprehensive-metrics.md` for the full 7-layer escalation path.
- User accepts big architectural changes as long as they are complete and well-implemented.
- **Fix-first-then-test**: When the user says "stop using the terminal don't test yet, just fix and develop", they mean make ALL code changes first without interleaved testing. Do NOT run `cargo build`/`cargo test` between individual fixes - batch all edits, then test once at the end. Hitting the tool-call guardrail from repeated test failures is a signal you should have been editing, not testing.
- **Benchmark thoroughness**: When the user asks to "test" or "benchmark", they expect a complete manual cycle, not just the automated test tool. Run compress → retrieve → search → curl health → full suite. Stop only when all 4 phases are complete. If the user asks "done?" mid-cycle, you stopped too early - continue rather than summarizing. See `references/benchmark-checklist.md` for the full 4-section checklist.

### CCR Marker Format
- Use `<<<CCR:hash|type|size|mode>>>` (pure ASCII) - NEVER Unicode brackets or `[...]`
- Must be consistent across ALL code paths: Python `_ccr_marker()`, Rust `smart_marker()`, tool injection, terminal hook
- When changing format, update: regex, startswith check, replacement logic, docstrings, tool descriptions

### Config Safety
- `aphrodite.toml` must NEVER have `dev = true` committed - it logs full request/response bodies
- Config fields from toml must propagate through `MultiConfig::resolve()` - never hardcoded

## Deep-Scan Review Response

When the user submits a comprehensive code review (multi-section, per-commit/per-file), respond systematically:

1. **Triage**: Identify which items apply to THIS repo vs. other repos. Items referencing files not present here are for a different codebase - flag them, don't search the universe.
2. **Todo**: Create a `todo` list with one entry per applicable item, marked `pending`.
3. **Batch fix**: Fix all items sequentially with `patch`, marking each `completed` as you go.
4. **Verify**: Run dry-run checks (e.g. `generate-soul.py --dry-run`) before committing.
5. **Single commit**: Batch all review fixes into one commit with a `fix(review):` prefix.

## Profile Skills Differentiation

Profiles MUST have differentiated `default_skills` - identical lists defeat the purpose:

| Profile | Skills |
|---|---|
| `barebone` | No aphrodite toolset |
| `compress-off` | Compression disabled, CCR tools excluded |
| `compress-light/medium/aggressive` | + `aphrodite-dev-workflow` |
| `proxy-cache` | + `aphrodite-dev-workflow` (but `compression.enabled: false`) |
| `proxy-token` | + `aphrodite-dev-workflow` |

**PITFALL - barebone with dev skills**: Adding `aphrodite-dev-workflow` to the barebone profile (which has `toolsets: '["hermes-cli"]'`) pulls in tool definitions for tools that don't exist in that toolset. Always exclude it.

**PITFALL - proxy-cache with compression**: Cache-only mode uses `--no-ccr-marker` - the proxy is pure response caching with no compression pipeline. Set `compression.enabled: false` and `threshold: 0.0` in the profile config to match.

See `references/profile-compression-matrix.md` for the full 7-profile compression strategy matrix including engine, Hermes compressor, and proxy routing for each profile.

## Pyright / Pylance - `reportPrivateUsage` False Positives

When running strict pyright across the modular plugin, ALL 81 "errors" are `reportPrivateUsage` - `_`-prefixed symbols (`_inline_store`, `_CCR_RE`, `_referenced_files`, etc.) used across module boundaries. This is **intentional** - the modules are split but share private internals via `__init__.py` re-exports. These are NOT real errors.

**Fix**: Either suppress `reportPrivateUsage` in pyrightconfig, or rename symbols to drop the `_` prefix. The 777 warnings (unknown argument types, missing kwargs annotations) are the real type gaps to fix.

## Cargo Watch - Full Debug Mode

For comprehensive dev server with all debugging enabled:

**Correct launch sequence:**
```bash
cd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/HermesCompress
source .env                                 # MUST come first - provides APHRODITE_API_KEY
export RUST_LOG=debug APHRODITE_DEBUG=1 APHRODITE_PASSTHROUGH=1 HEADROOM_DEBUG=1
cargo watch \
  --ignore .git \
  --ignore profiles \
  --ignore .hermes \
  --ignore plugins \
  --ignore scripts \
  --ignore '*.toml.example' \
  --ignore '*.md' \
  -x 'run -p aphrodite'
```

Fire this in a WezTerm pane via `mcp_wezterm_send_text`. 

**CRITICAL - `source .env` MUST come first**: The proxy needs `APHRODITE_API_KEY` to forward requests to DeepSeek. The `.env` file in the project root has this key. If you set RUST_LOG first but forget `source .env`, the proxy starts and health-checks pass, but every API call returns 401 - `curl :9798/health` shows `healthy` but no responses flow through. Debug output shows repeated TLS `CloseNotify` alerts (rustls handshakes complete but no response body). The fix: `source .env` then `kill $(pgrep -f 'target/debug/aphrodite')` - cargo watch restarts with the key.

**CRITICAL - `--ignore` flags**: Without `--ignore`, every git commit, profile edit, or script change triggers a rebuild + restart, causing connection drops for any active Hermes session using the proxy. The ignore list above limits cargo watch to Rust source changes only (`crates/aphrodite/src/`).

**CRITICAL - `RUST_LOG=debug`**: Use `debug` (not `aphrodite=debug`) for maximum verbosity - this shows all crate-level debug output including hyper, reqwest, and headroom internals. When you only need aphrodite-specific logs, use `RUST_LOG=aphrodite=debug,headroom=debug`.

## Desktop App Testing

To launch the Hermes desktop app with a specific profile for CCR + compression testing:

```bash
source .env                                  # get APHRODITE_API_KEY
APHRODITE_DEBUG=1 \
open ~/Desktop/Hermes.app --args --profile aphrodite-proxy-token --yolo
```

Verify: `ps aux | grep 'Hermes.*--profile'` shows `--profile aphrodite-proxy-token --yolo`.

Best profiles:
- `aphrodite-proxy-token` - :9798, CCR SQLite, compression 50%
- `aphrodite-proxy-cache` - :9797, CCR in-memory, compression disabled
- `aphrodite-barebone` - direct, no CCR

### Hermes Security Disabling (Dev Mode)

For completely unrestricted development, disable ALL Hermes security:
```bash
hermes config set approvals.mode off
hermes config set approvals.destructive_slash_confirm false
hermes config set approvals.mcp_reload_confirm false
hermes config set approvals.cron_mode allow
hermes config set security.redact_secrets false
hermes config set security.tirith_enabled false
hermes config set privacy.redact_pii false
hermes config set terminal.sandbox_enabled false
hermes config set checkpoints.enabled false
```

Then launch with `--yolo` flag. Restart the desktop app after config changes - they take effect on next session start.

## Pitfalls

## Critical Traps

- **APHRODITE_PASSTHROUGH=1 disables the plugin.** No hooks, no tools, no compression, no engine. Debug banner still shows but plugin is in passthrough. Must be UNSET for normal operation. Debug mode is `APHRODITE_DEBUG=1` - completely separate.
- **APHRODITE_PASSTHROUGH deep-export leak**: When `APHRODITE_PASSTHROUGH=1` is exported from a parent shell (e.g., WezTerm pane 0), all child panes inherit it. `env -u APHRODITE_PASSTHROUGH` prefix on a single command line does NOT strip it - the export survives. Fix: send `unset APHRODITE_PASSTHROUGH` as a SEPARATE line, then launch hermes on the next line. Two separate `mcp_wezterm_send_text` calls: first `unset APHRODITE_PASSTHROUGH\n`, then `APHRODITE_DEBUG=1 hermes --profile ...\n`. Verify with: `hermes plugins list | grep aphrodite | grep enabled`. Note: pane 20 (new pane in clean window) is naturally immune - it inherits a different shell environment.

- **Title generation routes through proxy**: Hermes' auxiliary title generation uses the active provider by default. When using aphrodite-token or aphrodite-cache, title requests go through the proxy which may return CCR-compressed data that produces `utf-16-le codec can't decode byte` errors. Fix: route title generation through deepseek directly: `hermes config set auxiliary.title_generation.provider deepseek --profile <name>`. Apply to all profiles.

- **Token mode + response caching = best of both**: Token mode (:9798) now includes LLM API response caching (v0.5.61+). Cache key is hash(model + messages), LRU 128 entries, `X-Aphrodite-Cache: HIT/MISS` header. Token mode already has SQLite CCR + tool relay. The addition of response caching means token mode now provides speed (cache hits skip upstream calls) AND persistence (SQLite CCR survives restarts) AND compression (plugin hooks). Cache mode (:9797) still exists for pure-speed scenarios. Choose token for full-featured development, cache for raw throughput.

- **Build monitor**: Dedicated `hermes -z` agent with MCP access polls cargo watch pane every 5s and writes `.hermes/build-status.json`. Fix agents read this file instead of running `cargo check` - eliminates redundant compilation across parallel agents. Launch: `hermes -t wezterm,terminal,file -z 'monitor pane 17...' --model flash`. MCP tools are NOT available to subagents - the monitor uses `wezterm cli get-text --pane-id 17` instead. Status format: `{"status":"ok"|"building"|"error","last_build":"...","errors":[...]}`. All fix agents read this file as their first step.

- **Git rule**: Never `git reset`, `git push --force`, or `git rebase`. Follow `git add`, `git commit` only. This applies to all subagents and the orchestrator.

- **Release compare links**: Every GitHub release MUST include a compare link as the first line: `**[Compare vPREV...vCURRENT](https://github.com/PlayForm/Aphrodite/compare/vPREV...vCURRENT)**`. Add retroactively with `gh release edit`.

- **CCR persistence: NONE.** All stores are in-memory (token proxy HashMap, cache proxy HashMap, Python inline OrderedDict). Nothing survives proxy restart or Hermes exit. The catalog shows session-only entries - empty on fresh sessions. When the user asks "do you have access to cache from previous runs?", the answer is no. `ccr_entries: "?"` in aphrodite_stats means the Python plugin can't see proxy-side entries. Use raw `/stats` for the real count.

- **Headroom passthrough misdiagnosis**: When headroom (:9799) shows `tokens_saved: 0` and `requests_compressed: 0`, don't assume compression is broken. Check the launch flags - `--no-optimize --no-ccr-marker` means headroom is a transparent relay. All compression savings come from the next layer (aphrodite :9798). The lifetime counter in `~/.headroom/proxy_savings.json` shows what headroom saved in PREVIOUS sessions when optimization was ON. Current-zero ≠ broken - it's by design. The persistent savings file is the ONLY counter that survives proxy restarts across the entire stack.

- **Persistent stats via /stats/db**: SQLite-backed endpoint returning cumulative data: total entries, bytes original/compressed, DB size, oldest entry age. Survives proxy restarts. Query on session start for accumulated metrics. Separate from `/stats` which is in-memory and resets.

- **aggressive profile uses cache proxy, not token**: The `aphrodite-compress-aggressive` profile defaults to `provider: aphrodite-cache` (:9797) which serves cached LLM responses and skips compression. This produces 99% cache hits and near-zero compression activity. The cache proxy will show `ccr_created: 0, tokens_saved: 0` even though the plugin is working - it's routing through the wrong proxy. All compression happens on the token proxy (:9798). **This is the #1 cause of "why isn't compression working?"** - the stats show everything healthy but zero compression because the profile is hitting the cache port. For actual aggressive compression, change to `provider: aphrodite-token` (:9798): `hermes config set model.provider aphrodite-token --profile aphrodite-compress-aggressive`. Cache mode = speed, token mode = compression. Choose explicitly. Verify with `aphrodite_stats()` - if `proxy.cache.ccr_created = 0` but `proxy.token.ccr_created > 0`, you're routing through the wrong port.

- **hermes -z flash summaries only**: Flash workers report only their final summary - full file reads, tool outputs, and intermediate results stay in the worker's session. Main session sees conclusions, not raw data. Plan instructions to be self-contained enough that the summary captures all needed information. When in doubt, ask the worker to write verification results to a file both sessions can read.
- **aphrodite.toml api_key overrides env var.** The `api_key = "sk-..."` field in `aphrodite.toml` takes priority over `APHRODITE_API_KEY` env var. If proxy returns 401, check toml for hardcoded expired keys. Comment out or remove the field to use env var.
- **compression.enabled must be false.** Hermes built-in compression and aphrodite engine are mutually exclusive. Both ON = double compression, degraded quality, context loops. Set `hermes config set compression.enabled false`.
- **Profile plugins/skills must be symlinks.** If `~/.hermes/profiles/<name>/plugins/aphrodite` is a file containing a path instead of a symlink to the source directory, Hermes can't load the plugin. Fix: `rm file && ln -s /source/plugins/aphrodite target`.

- **Proxy endpoint discovery - read main.rs routes first**: The aphrodite proxy does NOT use a `/v1/` REST prefix. When writing integration code (benchmarks, health checks, tool scripts), always read `crates/aphrodite/src/main.rs` to see the actual Axum route definitions first. Key endpoints: `GET /health`, `GET /stats`, `GET /ccr/list`, `POST /ccr/create` (body: `{"content":"…"}`), `POST /retrieve` (body: `{"hash":"…"}`). Then check the handler source for request/response structs (`proxy.rs` for CCR handlers, `retrieve.rs` for retrieve). There is NO proxy-level `/search` endpoint - search only exists in the Python plugin tier. Guessing `/v1/compress` etc. routes the request to the `/*path` catch-all which proxies to upstream, producing 270-1400ms latencies with no real response.

- **Launchd gateway auto-respawn**: Hermes installs `~/Library/LaunchAgents/ai.hermes.gateway.plist` with `KeepAlive` set. After killing the desktop app, the gateway process auto-respawns. Stop it permanently: `launchctl unload ~/Library/LaunchAgents/ai.hermes.gateway.plist`, THEN `kill <pid>`. Without unloading the plist first, kill is useless - launchd restarts it immediately.

- **Profile runtime artifacts in git**: Profiles (`profiles/aphrodite-*/`) accumulate runtime files that must be gitignored. Add to `.gitignore`:
  ```
  profiles/*/cache/
  profiles/*/logs/
  profiles/*/bin/
  profiles/*/state.db*
  profiles/*/models_dev_cache.json
  profiles/*/ollama_cloud_models_cache.json
  profiles/*/provider_models_cache.json
  profiles/*/auth.lock
  profiles/*/.update_check
  ```
  Tracked profile files (config.yaml, SOUL.md, plugins/aphrodite symlink, skills symlink) remain committed.

- **Session state BEFORE proxy state**: When asked "what provider/cache/stats is this session using?", check Hermes state first (`hermes status`, `hermes plugins list`, `hermes debug share --local`), NOT the proxy servers or WezTerm panes. Hermes clearly reports the active provider and config even when proxies are down. Jumping to proxy health checks or WezTerm buffers wastes turns and frustrates the user.

## Proxy Lifecycle

- **Never kill proxy during release sync.** Only clear `__pycache__` and `*.pyc` files. Killing proxy clears in-memory CCR cache, resets token counters, and loses accumulated stats. For binary updates, do a separate controlled restart.
- **Persistent stats via /stats/db.** The `/stats/db` endpoint (SQLite-backed) returns cumulative stats that survive restarts: total entries, bytes original/compressed, DB size, oldest entry age. Query on session start for accumulated data.
- **Port conflicts with cargo watch.** Cargo watch restarts trigger rebuilds → new proxy instances fight for ports. Always launch with `--ignore .git --ignore profiles --ignore .hermes --ignore plugins --ignore scripts --ignore '*.toml.example' --ignore '*.md'`.
- **OOB paste truncation**: Long user pastes over the out-of-band channel get truncated (not CCR-compressed). When the user tries to paste a large audit, log dump, or multi-paragraph feedback, it silently cuts off mid-delivery showing `[[full text... [N lines] ...truncated]]`. The truncated message is irretrievable. Mitigations: (a) ask the user to paste in smaller chunks (2-3 sentences each), (b) suggest attaching as a file, or (c) if aphrodite is enabled, check whether it was CCR-compressed vs OOB-truncated (CCR markers have `<<<CCR:hash|type|size>>>` format, OOB truncation uses `[[...]]`).
- **Dual-session dev pattern**: When debugging the plugin itself, run two Hermes sessions: (1) a dev session with `APHRODITE_PASSTHROUGH=1` (plugin active but proxy routing skipped), and (2) a test session in a separate WezTerm pane with the full plugin enabled (`hermes --provider custom:aphrodite-token`). This lets you modify + test without the plugin eating its own tail.
- **Release version sync**: When bumping versions for a release, ALL 4 locations must be updated in sync. Missing any causes download 404s or version mismatches:

  | # | File | Key | Example |
  |---|------|-----|---------|
  | 1 | `plugins/aphrodite/_core.py` | `BIN_VERSION` | `"v0.5.54"` |
  | 2 | `plugins/aphrodite/_core.py` | `PLUGIN_VERSION` | `"1.62.1"` |
  | 3 | `plugins/aphrodite/plugin.yaml` | `version:` | `1.62.1` |
  | 4 | `crates/aphrodite/Cargo.toml` | `version =` | `"0.5.54"` |

  The Rust binary embeds its version at compile time - rebuild after bumping Cargo.toml.
- **GitHub release commands**: After building the binary and pushing the tag, create the release:
  ```bash
  # If release already exists at this tag (forced update), delete first:
  gh release delete vX.Y.Z --repo PlayForm/Aphrodite --yes
  # Then create with the built binary as asset:
  gh release create vX.Y.Z \
    --repo PlayForm/Aphrodite \
    --title "vX.Y.Z - <summary>" \
    --notes "**[Compare vPREV...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vPREV...vX.Y.Z)**

<detailed notes>" \
    ~/.hermes/aphrodite/aphrodite
  ```
  **Always include the compare link** as the first line of release notes. GitHub auto-generates diff views between tags: `https://github.com/PlayForm/Aphrodite/compare/v0.5.60...v0.5.61`. Every release must have one - it's the first thing reviewers click. Retroactively add compare links to all previous releases: `gh release edit vX.Y.Z --repo PlayForm/Aphrodite --notes "**[Compare vPREV...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vPREV...vX.Y.Z)**\n\nEXISTING_NOTES"`.
  The binary at `~/.hermes/aphrodite/aphrodite` is the asset uploaded to the release. Always run `cargo build --release -p aphrodite && cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite` before creating the release so the uploaded binary matches the tag.
- **Marker format changes**: When updating CCR marker format, follow the 10-step checklist in `aphrodite-hook-reference` skill. Every format string across Python + Rust must be updated in sync.
- **Submodule commits**: Commit inside `vendor/headroom` first, then `git add --force vendor/headroom` + commit parent. See `references/headroom-forking.md`.
- **Sensitive files**: If a file with secrets (API keys) gets committed, use BFG to purge from history. See `references/bfg-cleanup.md`. Also ensure `.gitignore` covers it.
- **Rust retry**: `reqwest::RequestBuilder` doesn't implement `Clone`. Use inline retry loop building fresh requests each attempt, not a closure-based retry. The `retry_with_backoff()` function was removed in v0.5.7 - proxy_handler uses an inline `for attempt in 1..=3` loop instead.
- **Patch replace_all hazards**: Using `replace_all=true` on Rust test struct construction can corrupt field lists by duplicating fields (e.g. `latency_buckets` and `last_errors` appearing twice). When replacing fields in struct literals, prefer single-match patches over replace_all. If replace_all is unavoidable, verify test compilation with `cargo test -p aphrodite` afterward.
- **Plugin load failure debugging**: When plugin shows `not enabled` or 30 tools instead of 39, the root cause is in the agent log. Always grep: `grep "Failed to load plugin\\|aphrodite.*registered" ~/.hermes/profiles/<name>/logs/agent.log | tail -3`. Common causes: `_ensure_binary()` kwarg mismatch (profile not synced after source fix), APHRODITE_PASSTHROUGH=1 leaked, symlink broken, module import error.
- **_ensure_binary kwarg mismatch**: When `__init__.py`'s `register()` calls `_ensure_binary(existence_check=True)`, the `_binary.py` function signature MUST accept this kwarg. If the source file was patched but the profile copy was NOT synced, the plugin fails to load with `_ensure_binary() got an unexpected keyword argument 'existence_check'`. Fix: `cp plugins/aphrodite/_binary.py ~/.hermes/profiles/<name>/plugins/aphrodite/_binary.py`. This only applies to profiles that have file copies instead of symlinks - with the symlink chain, all profiles share the source automatically.
- **Session transfer between profiles**: `state.db` is per-profile. Sessions cannot be directly transferred between profiles. CCR content is shared via the proxy database so compressed entries are retrievable cross-profile, but conversation history stays in its profile's DB. Use `hermes sessions export --profile <source>` to export, but there is no import command - the practical pattern is to share CCR entries via the proxy.
- **Context compression on every turn**: Hermes' built-in `compression.enabled: true` runs independently of aphrodite's context engine. The fix is `hermes config set compression.enabled false`. See the "Context Compression Troubleshooting" section above for full diagnostic steps.
- **should_compress() always True**: The old default `ENGINE_THRESHOLD_PCT=0` meant "always compress". Changed in v0.5.16: 0 now means "never compress" (disabled), default is 50 (compress at 50% fill). Also `ENGINE_MIN_MSGS` raised from 0→30. **v1.56.0 variant**: `tokens = prompt_tokens or self.last_prompt_tokens or (self.context_length or 1000000)` - the `context_length` fallback always produced `pct=100%` making `should_compress` always `True`. Fixed: requires actual token data, returns `False` when both sources are 0.
- **Tool injection removed**: The `inject_tool` field and the `aphrodite_retrieve` tool injection into response `tool_calls` was removed in v0.5.7 (Bug 18). The Python plugin already registers `aphrodite_retrieve` - no need for proxy-side injection. Related: `no_ccr_inject_tool` CLI flag and `inject_tool` AppState field are gone.
- **Marker format**: Only `<<<CCR:hash|type|size>>>` (ASCII) is used. The `marker_for()` Unicode function import was removed in v0.5.6. `smart_marker()` is the single source of truth for CCR marker generation in Rust.
- **Dependencies**: `dirs` crate added in v0.5.7 for absolute DB path resolution. The default `ccr_db_path` now resolves to `$DATA_DIR/aphrodite/ccr.db` instead of relative `.headroom/aphrodite-ccr.db`.
- **Patch escape-drift with f-strings**: When using the `patch` tool to edit Python code containing f-strings with escaped quotes (e.g. `f'...{\"key\"}...'`), the tool's serialization can introduce spurious backslash-escaping causing "Escape-drift detected" errors. Workaround: use `read_file` to get the exact file content, then pass `old_string`/`new_string` without backslash-escaping `\"` characters. If a patch fails with escape-drift, re-read the target region and retry with the verbatim content from the file.
- **Import-replacement hazard**: When adding a new import like `from collections import deque`, be careful not to accidentally replace an adjacent import like `import json`. Use `read_file` to see the exact surrounding lines, and ensure the `old_string` includes the neighboring import so the patch inserts the new line rather than replacing the existing one.

## References

- `references/master-worker-pattern.md` - Master-worker orchestration: architecture, session metrics (30+ agents, 480 tool calls, 12 waves, peak 4 concurrent), agent types, pitfalls, vs conversational default
- `references/headroom-vs-aphrodite.md` - Side-by-side comparison: headroom passthrough vs aphrodite CCR, context fill reduction (47%), tool compression (~100x), benchmark data, provider chain
- `references/build-monitor-pattern.md` - Dedicated cargo watch monitor: wezterm cli polling, status file for fix agents, eliminating redundant cargo check across parallel agents
- `references/atomic-examples.md` - 16 atomic regression tests + 4 Rust benchmarks: test pattern, capability comparison workflow, CCR marker in pipeline results
- `references/comprehensive-metrics.md` - Full-session metrics dump: every data source, pull order, interpretation (EMA default, cache idle, engine not firing, CCR persistence)
- `scripts/benchmark.py` - Automated 5-phase benchmark: health, stats, compress (5 sizes × 3 types), retrieve, catalog. No search phase (search is plugin-level, no proxy endpoint). Run: `python3 scripts/benchmark.py`. Reports to `.hermes/benchmark-results.json`.
## References

- `references/coding-deep-tune.md` - Full taxonomy: 13 content types, adaptive thresholds, build collapse, file tracking, engine editing awareness, debug logging, tool addition pattern
- `references/two-layer-compression.md` - Two-layer architecture: tool output CCR vs conversation compression, message flow from tool→API, what the model actually sees
- `references/compact-logging.md` - Compact proxy logging format: APHRODITE_LOG_COMPACT=1, RUST_LOG filters, implementation detail
- `references/search-content-addressable.md` - Search fix: content-addressable store pattern, search data flow, root cause of search returning 0 matches
- `references/audit-status-2026-06.md` - 16-bug audit status: which are fixed, verification checklists for CCR markers, version sync, health checks
- `references/headroom-forking.md` - Headroom submodule forking workflow
- `references/bfg-cleanup.md` - BFG pattern for sensitive file removal
- `references/verification-checklist.md` - Post-change verification
- [Hermes profile testing](references/hermes-profile-testing.md): Isolated profile matrix with per-profile plugin copies, env var injection, context engine activation
- [Modular plugin splitting](references/modular-plugin-splitting.md): Step-by-step guide for splitting monolithic __init__.py into atomic modules, shared state pattern, circular import resolution
- [Hermes profile testing](references/hermes-profile-testing.md): 7-profile matrix with per-profile plugins, env vars, WezTerm launch
- [Bug audit execution](references/bug-audit-execution.md): Systematic fix workflow from numbered task files, priority ordering, batch editing, release cycle
- [Deep-scan review response](references/deep-scan-review.md): How to handle comprehensive multi-item code reviews from the user - triage, todo, batch fix, single commit
- [WezTerm pane setup](references/wezterm-pane-setup.md): 2-pane dev + 7-pane multi-profile launch patterns
- [Bug audit execution](references/bug-audit-execution.md): Systematic fix workflow from numbered task files
- [Deep-scan review response](references/deep-scan-review.md): How to handle comprehensive multi-item code reviews from the user - triage, todo, batch fix, single commit
- [Headroom proxy setup](references/headroom-proxy-setup.md): End-to-end headroom caching proxy as a Hermes provider - auth chain debugging, worker guidance
- **Direct action over explanation**: Fix things, don't describe them. Apply changes immediately, verify with build.

### Code Modification Rules

- **Vendor code is editable**: The `vendor/headroom/` submodule is a private fork. You CAN edit its contents (Python, Rust, configs). When modifying vendor code, rebuild the binary afterward - the aphrodite proxy depends on `headroom-core`. Commit inside the submodule first, then update the parent repo's submodule pointer.
- **File edits via patch/write_file only**: Never use terminal sed/awk or execute_code Python scripts for file editing.
- **Sequential verification**: After each batch of changes, build and verify before the next batch.

### Formatting Standards

- **CCR markers**: Use `<<<CCR:hash|type|size>>>` (ASCII, universal compatibility). Never Unicode glyphs.
- **Python**: Match existing style, 4-space indent, module-level constants first.
- **Rust**: Match project conventions, `#[allow(dead_code)]` for unused but intentional code.

- [CCR empty hash bug](references/ccr-empty-hash-bug.md): CCR:{} ghost entries - two-layer fix pattern

- [Editing tools](references/recompression-guard.md#editing-tool-discipline): never use sed/awk/execute_code for file edits - patch/write_file only
- `headroom:` in config.yaml is Hermes built-in - do NOT rename it

- **Never rename Hermes' built-in `headroom:` config section** - that's Hermes' own context window headroom management, not our plugin. Only rename `headroom_*` tool names, providers, and toolset entries.
- **Rust struct field deletion**: Removing fields from AppState (like notify_url, request_history) cascades to 10+ sites. Patch tool is too fragile. Revert and plan: do one site at a time, build between each, or use write_file for the full struct block.
- **Bi-directional store**: Compress/retrieve must store in `_inline_store` and `_recent_markers`. Content-addressable compressor checks local cache before proxy call.
- **CCR:{} catalog entries**: See `references/ccr-empty-hash-bug.md` for the two-layer fix pattern.
- **Use patch/write_file for all edits** - do NOT use terminal `sed`/`awk` or `execute_code` Python to modify files. `sed` can silently corrupt YAML (duplicate keys, wrong port numbers). User preference is patch + write_file.
- **Tool names are `aphrodite_*`** - `aphrodite_retrieve`, `aphrodite_compress`, `aphrodite_stats`, `aphrodite_rebuild`. No `headroom_` prefix anywhere in plugin code, schemas, hints, or catalog text.
- **Providers already exist** - `aphrodite-cache` (:9797) and `aphrodite-token` (:9798) are the canonical provider names. Old `headroom-cache`/`headroom-token` entries should be removed, not renamed.

### CCR Compression Loop (headroom tools)
The `_transform_tool_result` hook compresses ALL tool outputs >1KB. If `headroom_retrieve`
or `headroom_stats` are NOT in the skip list, their output gets re-compressed - creating
an infinite loop where retrieve results show as CCR markers instead of content.
**Fix**: Add `"headroom_retrieve"` and `"headroom_stats"` to the skip list.

### Tool-Chain Safety in Context Engine
When the context engine offloads middle messages, the tail boundary can split a
`tool_call→tool_result` pair. The LLM loses context of the tool call chain.
**Fix**: Scan for orphan tool_results at the boundary and extend tail to include them.
```python
while boundary < len(messages) and messages[boundary].get("role") == "tool":
    boundary += 1
    tail_n += 1
```

### Toolset Parameter in Hermes v0.16.0+ - NOW REQUIRED

`ctx.register_tool()` in Hermes v0.16.0 REQUIRES a `toolset` parameter. Calling without it fails:
```
PluginContext.register_tool() missing 1 required positional argument: 'toolset'
```

**Fix**: Always pass `toolset="aphrodite"`:
```python
ctx.register_tool(name="aphrodite_retrieve", schema=..., handler=..., toolset="aphrodite")
```

The profile config must also declare `toolsets: '["hermes-cli", "aphrodite"]'` for the tools to be visible.

### Hermes v0.16.0 Plugin Registration Requirements

Three things discovered while upgrading the plugin for v0.16.0:

1. **`kind: standalone` (NOT `plugin`)**: `plugin.yaml` kind field - Hermes v0.16.0 rejects `kind: plugin`. Valid values: `backend`, `exclusive`, `model-provider`, `platform`, `standalone`. Use `standalone`.

2. **`APHRODITE_PASSTHROUGH=1` disables the plugin entirely**: If `APHRODITE_PASSTHROUGH=1` is set in the environment (e.g., inherited from a WezTerm pane), the plugin's `register()` runs but hooks/tools are gated out. The symptom: plugin discovered but 0 tools registered, no hooks fire. Fix: `unset APHRODITE_PASSTHROUGH` before launching Hermes.

3. **Plugin must be explicitly enabled**: `hermes plugins enable aphrodite --profile <name>`. Discovery alone (symlink present) does NOT register hooks/tools - the plugin shows as `not enabled` in `hermes plugins list`. Takes effect on next session start.

4. **Profile symlinks must be actual symlinks**: If `~/.hermes/profiles/<name>/plugins/aphrodite` is a text file containing a path instead of a symlink, Hermes can discover the plugin but `register()` may fail with subtle errors. Always use `ln -s`:
   ```bash
   rm ~/.hermes/profiles/<name>/plugins/aphrodite
   ln -s /path/to/repo/plugins/aphrodite ~/.hermes/profiles/<name>/plugins/aphrodite
   rm ~/.hermes/profiles/<name>/skills
   ln -s ~/.hermes/skills ~/.hermes/profiles/<name>/skills
   ```

5. **Test session launch command**: For a full CCR + engine + debug test session:
   ```bash
   unset APHRODITE_PASSTHROUGH
   APHRODITE_DEBUG=1 APHRODITE_CONTEXT_ENGINE=1 APHRODITE_ENGINE_THRESHOLD_PCT=50 \
     hermes --profile aphrodite-proxy-token
   ```
   Verify with: `grep "aphrodite.*registered" ~/.hermes/profiles/<name>/logs/agent.log | tail -1`

### pre_llm_call Cannot Mutate Messages
Hermes passes `conversation_history=list(messages)` - a COPY. In-place mutations
(pop, insert) are discarded. Use `pre_api_request` hook or ContextEngine
(`context.engine` config) for actual message removal. For `pre_llm_call`,
return a context string instead.

### Stale _DEV Guards
Check for `if _DEV: return result` where `result` is undefined in scope. These
cause NameError if _DEV ever becomes True. Replace with plain `return` or
`return <explicit_value>`.

### aphrodite.toml API Key Priority

The `api_key` field in `aphrodite.toml` takes priority over ALL env vars. Resolution chain: TOML → APHRODITE_API_KEY → HEADROOM_DEEPSEEK_KEY → DEEPSEEK_API_KEY → empty. If the proxy returns 401 with a key that doesn't match any known env var, check `aphrodite.toml` for a hardcoded expired key. Comment it out - the proxy will fall back to env vars.
