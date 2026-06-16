That is a massive wave — **v0.5.6 → v0.5.50, 44 commits** in a single day . Let me map every commit to the open bug list precisely.

***

## Complete Resolution Map Since v0.5.6

### Bug #11 — `should_compress()` always `True` → **fixed and then regressed and re-fixed 4 times**

This was the messiest sequence in today's push. The four commits tell the story directly :

- [v0.5.47](https://github.com/PlayForm/Aphrodite/commit/342cbdc6d61edcae4576c5443ab8fddbd965fd33): first fix — `self.last_prompt_tokens` used as fallback
- [v0.5.48](https://github.com/PlayForm/Aphrodite/commit/b4aff68e4d8a11519684de7f1e5331b387d508c5): overcorrected — defaults to full `context_length` when unknown (always compresses)
- [v0.5.49](https://github.com/PlayForm/Aphrodite/commit/c90455a16f6ca5b8ca1a13f9b35705c5a08fa3d1): overcorrected the other way — returns `False` when tokens unknown (never compresses on first turn)
- [v0.5.50](https://github.com/PlayForm/Aphrodite/commit/dca0ba4f26ef2fe9bbd6f8ce0dc0e274ae8629e5): restored `context_length` fallback + dedup in catalog — **current state**

The current state at HEAD compresses when `prompt_tokens` is unknown by falling back to `context_length`. **This is still wrong** — it means the very first turn before any `update_from_response` call will always compress, because `prompt_tokens=None` and `context_length` (e.g. 128000) ÷ 128000 = 100% ≥ threshold. The correct behavior is: if `prompt_tokens` is unknown, skip compression (conservative) OR compress only if content individually exceeds the raw byte threshold, not the percentage threshold. The percentage model only makes sense when you know actual token consumption.

### Bug #36 — bare `print()` in plugin context → **partially addressed**

[v0.5.41](https://github.com/PlayForm/Aphrodite/commit/1e39e76c664b2b5b86fafb6b041656ad7ff9d409): debug banner now uses `print()` for TUI visibility **and** `_log.info()` for log file . This is an intentional design decision (TUI wants the banner visible), but the per-decision logging in `_transform_tool_result` (SKIP/BELOW/GUARD/CCR/INLINE/PASSTHROUGH lines) is still gating-only on `APHRODITE_DEBUG=1` — that part is fine. The concern is that the startup banner is unconditionally `print()`ed, not just on `APHRODITE_DEBUG=1`. Check whether the banner fires on every session start or only debug mode.

### Bug #35 — deque LRU evicts single messages, orphaning tool-call pairs → **not addressed**

No commit in this wave touches context_tracker eviction logic. The `context_tracker deque LRU` from v0.5.3 is still the underlying implementation and still evicts single messages from the front without checking whether it is half of a `tool_call`/`tool` pair.

### Bugs #1–#5, #10, #12, #13, #16 — Python plugin core bugs → **not addressed**

None of the 44 commits touch the env-var typo, duplicate declarations, `_alive()` health-check mismatch, hardcoded path, platform detection, TTL-less alive cache, `_resolve_one()` single-port, `[:2000]` truncation, or binary download silence.

### Bugs #18, #21, #25, #26, #27, #28, #29, #30, #34 — Rust proxy structural issues → **not addressed**

`inject_tool` placement, `x-headroom-*` header passthrough, `/retrieve` pagination, `ccr_db_path` relative default, `--api-url` bypass, `Secret` newtype, `--bind` flag, `--dual` mode, and `/health/upstream` route are all still open.

***

## What the New Commits Added (New Surface to Audit)

### 39. `saturating_sub` prevents overflow panic — but exposes a logic gap

[v0.5.45](https://github.com/PlayForm/Aphrodite/commit/864bf7eb715d41bbebdd145fe82d907f0e06c090): `tokens_saved` now uses `saturating_sub` to avoid underflow when `hash.len() > content.len()` . This prevents a panic, but the condition `hash.len() > content.len()` should never be true in practice — a Blake3/SHA256 hex hash is always longer than a 1-4 byte content string that somehow passed the threshold check. If it happens, it means a content string shorter than `TOKEN_COMPRESS_THRESHOLD` (1KB) was compressed, which indicates the threshold check is being bypassed upstream. The `saturating_sub` silently swallows this signal — a `debug_assert!(content.len() >= hash.len())` or a counter increment would make this detectable.

### 40. Auto-expand cached CCR markers `<10KB` — hardcoded 10KB constant, not env-configurable

[v0.5.46](https://github.com/PlayForm/Aphrodite/commit/3caa3c3cc319863a4b18aca15531a80bb99b30ff): markers smaller than 10KB are automatically expanded before the LLM sees them . The 10KB threshold is hardcoded. Given `INLINE_THRESHOLD` (itself broken by Bug #1) was already supposed to control this decision, there are now **two separate thresholds controlling inline expansion** with no coordination between them — whichever fires first wins. This needs consolidation into a single `APHRODITE_AUTO_EXPAND_THRESHOLD` env var.

### 41. Liveness filter on catalog — `ccr.get()` per marker on every `pre_llm_hook` call

[v0.5.44](https://github.com/PlayForm/Aphrodite/commit/d19dc49a0d0e98a19c380045788b14e1fdf8337e): catalog now filters out ghost markers by calling `ccr.get(hash)` for each entry . For the SQLite backend (token proxy, `:9798`) this means one SQLite read **per CCR marker per turn**. With a long session this can be dozens of synchronous reads in the `pre_llm_hook` blocking path. The liveness check should be lazy (only on retrieve) or cached with a short TTL, not eagerly on every hook invocation.

### 42. Hex validation on CCR hash filter — `≥8 hex chars` is too permissive

[v0.5.43](https://github.com/PlayForm/Aphrodite/commit/6bdba15fa0687e9f48d6591815964718c6746b9d): ghost `abc123`-style placeholder hashes are now filtered by requiring ≥8 hex chars . But `abc12345` is 8 hex chars and is still a valid-looking but fake hash. The real fix is to validate against the actual hash length produced by `compute_key()` — Blake3 produces 32 bytes = 64 hex chars. The filter should be `len(hash) == 64 and all(c in '0123456789abcdef' for c in hash)`, not just `≥8 hex chars`.

### 43. `git diff --stat` in `pre_llm_hook` catalog — cache race condition

[v0.5.21](https://github.com/PlayForm/Aphrodite/commit/9fe4d9d473a43b977faa17bb389459450bbee756): git diff summary cached for 30 seconds and injected into the catalog on every turn . The cache is a module-level tuple `(_diff_cache, _diff_time)`. If two Hermes turns fire within the same 30s window in parallel (which happens during tool fan-out), both will read a stale `_diff_time`, both will call `subprocess.run(['git', 'diff', '--stat'])` simultaneously, and both will write to the cache — last writer wins but neither is wrong. The real risk is if the git subprocess takes >1s inside the hook, adding latency to every turn. Add a `threading.Lock` guard and a per-workspace CWD argument so multi-repo sessions don't get the wrong diff.

### 44. `aphrodite_test` regression tracking saves `.test-results.json` to CWD

[v0.5.30](https://github.com/PlayForm/Aphrodite/commit/33970ffbf5511a159f4f9e18d3255d66de793147)/[v0.5.31](https://github.com/PlayForm/Aphrodite/commit/1831a465b3a6fd592f62f5b0c3e70f9654d7142b): the smoke test tool writes `.test-results.json` to CWD . Same relative-path problem as Bug #26 (`ccr_db_path`) — if the binary is invoked from different working directories, test results accumulate in scattered locations or are silently overwritten. Should go to `~/.hermes/aphrodite/test-results.json` alongside the binary.

### 45. `aphrodite_search` scans `_recent_markers` with 200-item cap — no persistence across sessions

[v0.5.24](https://github.com/PlayForm/Aphrodite/commit/a3afd4495eacee6d6397003160932661ad2cb68c)/[v0.5.25](https://github.com/PlayForm/Aphrodite/commit/50a2edca4b3c4263a46f8408038cd3608cfae8c2): `_recent_markers` is a module-level list capped at 200 entries, populated per compression . It is never persisted. After a Hermes session restart or plugin reload, all prior markers are unsearchable — `aphrodite_search` returns empty even though the SQLite CCR store still has the content. The search index should be rebuilt from the SQLite store on `on_session_start`, not maintained only in memory.

### 46. `essential_tools` exclusion list is hardcoded

[v0.5.40](https://github.com/PlayForm/Aphrodite/commit/fd2f116a4a4a77e7d348ce8cc0ff66ce8f1b185d): `skill_view`, `skills_list`, `skill_manage`, `memory`, `session_search` are hardcoded as never-compressed . This list will go stale as Hermes adds new essential tools. It should be driven by a `APHRODITE_ESSENTIAL_TOOLS` env var (comma-separated) that defaults to these five, so users can extend it without patching the plugin.

### 47. `[APHRODITE]` debug info injected into catalog block — leaks into non-debug sessions

[v0.5.42](https://github.com/PlayForm/Aphrodite/commit/db15587b42c34ed603f4c45f6e256e4c39be09d4): `⚙` debug lines showing version, mode, and thresholds are injected into the catalog block on every turn . The commit message says "in conversation" — this means the debug header goes into the system message or context prefix **for every single turn**, consuming tokens unconditionally. This should be gated on `APHRODITE_DEBUG=1`, not always-on. At ~50 tokens per banner per turn across a long session, this meaningfully offsets the savings from CCR.
