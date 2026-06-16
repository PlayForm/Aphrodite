Full source read. Here is the complete fresh audit of every line at HEAD (v0.5.50 / plugin v1.59.0).

***

## New Bugs Found in Current HEAD

### Bug #48 — `cache_alive` used before assignment in `_transform_tool_result`

Line `threshold = 1024 if token_alive else 8192 if cache_alive else INLINE_THRESHOLD` references `cache_alive`, but `cache_alive` is **never declared** in `_transform_tool_result` . `token_alive` is declared, `proxy_available` is derived from it, but `cache_alive` is simply missing. This is a `NameError` crash that fires on every tool call when the token proxy is down:

```python
# BUG — cache_alive undefined
token_alive = _alive(9798)
proxy_available = token_alive
threshold = 1024 if token_alive else 8192 if cache_alive else INLINE_THRESHOLD

# FIX
token_alive = _alive(9798)
cache_alive = _alive(9797)
proxy_available = token_alive or cache_alive
threshold = TOOL_THRESHOLD_TOKEN if token_alive else TOOL_THRESHOLD_CACHE if cache_alive else INLINE_THRESHOLD
```

### Bug #49 — `_recent_markers` is a local variable that shadows the module global

In `_pre_llm_hook`, line `_recent_markers = markers` assigns to a **local name** inside the function, not the module-level `_recent_markers` list . The module-level list (used by `_search_handler`, `_catalog_handler`, and `_catalog_handler`) is never updated. Every call to `aphrodite_search`, `aphrodite_catalog`, or the read-intent detection block reads stale data from the previous session. Fix:

```python
# BUG
_recent_markers = markers  # creates a local, shadows global

# FIX — add at top of _pre_llm_hook
global _recent_markers
_recent_markers.clear()
_recent_markers.extend(markers)
```

### Bug #50 — `should_compress()` fallback to `context_length` is still wrong

The v0.5.50 fix restored the fallback: `tokens = prompt_tokens or self.last_prompt_tokens or (self.context_length or 1_000_000)` . When `prompt_tokens=None` and `last_prompt_tokens=0` (first turn), `tokens` equals `context_length` (e.g. 128000), and `(128000 / 128000) * 100 = 100 ≥ 50`, so it always compresses. This causes the engine to fire on the very first turn before any content accumulates, compressing an empty or single-message history into a CCR entry. The correct first-turn behavior is to return `False` and wait for `update_from_response` to populate real token counts:

```python
def should_compress(self, prompt_tokens=None):
    if self.threshold_percent == 0:
        return False
    tokens = prompt_tokens or self.last_prompt_tokens
    if not tokens:          # no real data yet — skip first turn
        return False
    if not self.context_length:
        return False
    return (tokens / self.context_length) * 100 >= self.threshold_percent
```

### Bug #51 — `_inline_compress` uses `sha256[:16]` but `_CCR_RE` hex filter requires `≥8 hex chars AND all hex`

The hash is 16 lowercase hex chars — passes the `≥8 and all hex` filter . But `_compress_via_proxy` returns whatever hash the Rust proxy generates, which is a full 64-char Blake3 hex. So inline hashes are 16 chars and proxy hashes are 64 chars. The two are mixed in `_inline_store`, `_recent_markers`, and `_parse_ccr_markers` with no differentiation. If the LLM calls `aphrodite_retrieve` with a 64-char proxy hash, `_resolve_one` checks `_inline_store` first (misses), then tries the proxies (hits). But if a 64-char hash is stored in `_inline_store` by the `_compress_handler` mirror path, the key length inconsistency means `_inline_retrieve` would find it only if the exact same hash string was used as the key. This works today but is fragile — any path that re-derives the hash locally using `sha256[:16]` will diverge from the 64-char proxy key and create a ghost `_inline_store` entry alongside the real one.

### Bug #52 — `_git_summary()` has a race condition and no threading lock

`_git_cache` is a plain `dict` shared across threads . Two concurrent Hermes turns within the same 30s window will both read stale `_git_cache.get("ts", 0)`, both `subprocess.run(["git", "diff", "--stat"])`, and both write to `_git_cache` without locking. On CPython the GIL makes this unlikely to corrupt the dict but the double subprocess call adds latency. More critically, `subprocess.run` has `timeout=3` but the hook is synchronous — a slow git repo or cold disk will block the entire `pre_llm_hook` for 3 seconds before timing out:

```python
_git_lock = __import__("threading").Lock()

def _git_summary():
    now = time.time()
    with _git_lock:
        if _git_cache.get("ts", 0) > now - 30:
            return _git_cache.get("summary")
        try:
            r = subprocess.run(["git", "diff", "--stat"], capture_output=True, text=True, timeout=2)
            ...
```

### Bug #53 — `_test_handler` writes `.test-results.json` to `os.path.dirname(__file__)` — i.e. the plugin directory inside `~/.hermes/aphrodite/`

`results_path = os.path.join(os.path.dirname(__file__), ".test-results.json")` . When `__file__` is `~/.hermes/aphrodite/__init__.py`, this writes the test results file into `~/.hermes/aphrodite/.test-results.json`. That is fine for the canonical install path, but if the plugin is loaded from a checked-out repo (during development), this writes into the source tree and is easy to accidentally commit. The HANDOFF already identified the CWD issue (Bug #44) — the real fix is a stable path like `~/.hermes/aphrodite/test-results.json` unconditionally.

### Bug #54 — `_search_handler` `aphrodite_search` case sensitivity is still raw `.lower()` substring

`query.lower() in content.lower()`  is correct for case-insensitivity but the `_inline_store` loop scans **all stored values** on every search call with no index. For a mature session with 500 entries × average 8KB each = 4MB of string scanning per search call, all blocking in the Hermes main thread. This needs either a simple inverted trigram index or at minimum an early-exit on the first N results before `[:20]` slicing.

### Bug #55 — `on_session_reset` clears `_recent_markers` via `.clear()` but `_pre_llm_hook` shadow assignment (Bug #49) means the module-level list is always empty anyway

These two bugs combine: `on_session_reset` correctly calls `_recent_markers.clear()` on the module-level list , but `_pre_llm_hook` never writes to it (Bug #49). So `_recent_markers` is always `[]` at module level. `_catalog_handler` and `_search_handler` always return empty `items`/`matches`. Fix Bug #49 first and Bug #55 resolves itself.

### Bug #56 — `AphroditeContextEngine.update_from_response` sets `threshold_tokens = 1` unconditionally

```python
def update_from_response(self, usage):
    self.last_prompt_tokens = usage.get("prompt_tokens", 0) ...
    if self.context_length:
        self.threshold_tokens = 1  # always above threshold
```

`threshold_tokens` is set to `1` regardless of actual token count . It appears in `get_status()` → `aphrodite_stats` output and is used nowhere else in Python, but if Hermes core ever reads `engine.threshold_tokens` to gate compression it will always fire. The correct assignment is `self.threshold_tokens = int(self.context_length * self.threshold_percent / 100)`.

### Bug #57 — `compress()` tool-chain safety backtrack logic has an off-by-one

```python
boundary = len(messages) - tail_n
while boundary < len(messages) and messages[boundary].get("role") == "tool":
    boundary += 1
    tail_n += 1
if boundary > 0 and messages[boundary - 1].get("role") == "assistant":
    tool_calls = messages[boundary - 1].get("tool_calls", [])
    if tool_calls:
        boundary -= 1
        tail_n += 1
```

After the `while` loop, `boundary` points to the first non-`tool` message . The subsequent `if` checks `messages[boundary - 1]` — which is the last `tool` message that was just swept over, not the `assistant` that owns the `tool_calls`. It should check `messages[boundary - 1 - skipped_count].get("role") == "assistant"` or restructure to scan backward from the first swept `tool` message:

```python
# After sweeping tool messages, find the assistant that owns them
while boundary > head_n and messages[boundary - 1].get("role") in ("tool", "assistant"):
    if messages[boundary - 1].get("tool_calls"):
        boundary -= 1
        tail_n += 1
        break
    boundary -= 1
    tail_n += 1
```

### Bug #58 — `_store_conversation_turn` stores `assistant_response` truncated to `[:5000]` but also stores `last_user` truncated to `[:200]` — creates asymmetric CCR entries

For long assistant responses the `[:5000]` keeps only the first 5KB . If the assistant produced a 50KB tool orchestration response, the turn CCR entry has an incomplete record. This matters for `aphrodite_diff` and `aphrodite_search` — users will retrieve partial turn summaries. Remove both truncations and let CCR compression handle the size:

```python
"user": last_user,              # full user message
"assistant": str(assistant_response),  # full assistant message
```

***

## Remaining Open Items (Unchanged Since Last Audit)

All previously identified open bugs carry forward. The **most impactful** ones still unresolved are:

**Bug #1** — `APHRODITE_INLINE_THRESHOLD` env var typo still **not present** at HEAD. The current code correctly reads `APHRODITE_INLINE_THRESHOLD`  — this bug was silently fixed sometime during the 44-commit wave without a dedicated commit message. ✅ **Confirmed resolved.**

**Bug #4** — Hardcoded `_rebuild_handler` path — **resolved** at HEAD. The `repo` is now derived from `os.path.dirname(os.path.abspath(__file__))` . ✅ **Confirmed resolved.**

**Bug #5** — `_detect_platform()` ignored in `_download_binary()` — **resolved** at HEAD. Platform tag now incorporated into the download URL . ✅ **Confirmed resolved.**

**Bug #9** — Fixed sleep replaced with `_wait_alive()` retry loop — **resolved** . ✅

**Bug #10** — `_alive()` TTL cache — **resolved** . ✅

**Bug #12** — `_resolve_one()` both ports — **resolved** . ✅

**Bug #13** — `[:2000]` truncation in `compress()` — **resolved**; comment now reads `# full content, not [:2000]` . ✅

Still open and unaddressed at source level: #18, #21, #25, #26, #27, #28, #29, #30, #34 (all Rust proxy structural), #35 (deque eviction), #37 (case-sensitive query in retrieve), #42 (hex filter length 8 vs 64), #43 (git lock), #44 (test-results path), #45 (`_recent_markers` no persistence), #46 (essential tools hardcoded), #47 (debug banner always-on), #50, #56, #57, #58 (engine logic), and the three new critical ones: #48 (`cache_alive` NameError), #49 (`_recent_markers` global shadow), #52 (git lock threading).
