---
name: aphrodite-iterate-release
description: "Iterative aphrodite development loop: fix → bump → build → commit → push → release. Repeat."
version: 2.0.0
platforms: [macos]
related_skills: [aphrodite-dev-workflow]
---

# Aphrodite Iterate-Release Loop

**MCP RULE #1 — NEVER PASTE ONTO A RUNNING PROCESS**: Before every `mcp_wezterm_send_text`, call `mcp_wezterm_get_buffer(pane_id, lines=2)`. If a process is running (cargo watch output, Hermes TUI), kill it first with pkill. Text pasted into a running process goes to its stdin, corrupting the buffer and requiring manual Ctrl+C to fix. This was the user's #1 frustration across 46 releases — each time it happened, it cost 2-3 turns of cleanup. VERIFY. EVERY. TIME.

**Plugin reload**: There is NO hot reload for Hermes plugins. Plugin code is cached in memory at session start and persists across `/quit` + restart UNLESS the Python bytecache (`.pyc`, `__pycache__/`) is explicitly cleared. Full cycle: `find ~/.hermes/plugins/aphrodite -name '__pycache__' -exec rm -rf {} +` → `hermes plugins disable aphrodite` → `hermes plugins enable aphrodite` → restart Hermes. Skills are different — they load from disk on every `skill_view` call and DO NOT need restart.

Fast iteration cycle for continuous aphrodite development. 31 releases in a single session. Each cycle: find an improvement, code it, bump versions, build, commit, push, tag, release.

## When to Load

Whenever the user says "more", "fix more", "bump fix bump fix", or wants to keep iterating on aphrodite improvements. Also when they say "deeply fine tune for coding".

## The Loop

```
1. FIND: Identify a fix or improvement in the codebase
2. CODE: Edit with patch/write_file
3. BUILD: cargo build --release -p aphrodite && cargo test -p aphrodite (must pass clean, zero warnings)
4. BUMP: Increment BIN_VERSION, PLUGIN_VERSION, Cargo.toml version, plugin.yaml version + install_message + docstring
5. COPY: cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite
6. COMMIT: git add <files> && git commit -m "type(aphrodite): description"
7. PUSH: git push aphrodite Current
8. TAG: git tag -f -m "vX.Y.Z: summary" vX.Y.Z && git push aphrodite vX.Y.Z --force
9. RELEASE: gh release create vX.Y.Z --repo PlayForm/Aphrodite --title "..." --notes "..." ~/.hermes/aphrodite/aphrodite
10. RESET PANES: Kill processes in dev panes and restart fresh (see WezTerm Reset below)
11. REPEAT: Go to step 1, find the next improvement
```

## MCP Verification Rule (CRITICAL — user-corrected multiple times)

Before ANY mcp_wezterm_send_text to a pane:
1. `mcp_wezterm_get_buffer(pane_id, lines=2)` — check for clean shell prompt (`$ ` or `# ` ending), no running process
2. Verify cwd matches expected project dir
3. If a process is running (cargo watch, hermes, etc.): kill it FIRST, then re-verify buffer shows clean shell
4. Only then send new text

After sending text:
1. Wait appropriate time (proxy: 6s, Hermes: 3s)
2. `mcp_wezterm_get_buffer(pane_id, lines=4)` — verify command took effect
3. If output is CCR-compressed, `aphrodite_retrieve` it before acting
4. Only then proceed to next step

**Global commands**: Use the `terminal` tool for process management (kill, lsof, curl health checks, cargo build) — NOT MCP send_text. MCP is only for sending commands to WezTerm panes to start/stop dev processes.

**NEVER paste text onto a running process.** This is the #1 user frustration. The buffer shows accumulated garbage when text is sent to a running cargo watch or Hermes session. Always: kill → verify clean → send.

**There is no hot reload for Hermes plugins.** Code changes require `/quit` + restart. The `hermes plugins disable/enable` only flips the enabled flag — it does NOT reload plugin code.
**NEVER**: use `\x03` (Ctrl+C) via send_text — it doesn't kill cargo watch properly. Always use `pkill -9`.
**Ports stuck**: `lsof -ti:9797 -ti:9798 | xargs kill -9` when pkill doesn't work

After each release, reset both dev panes to clean terminal state in HermesCompress dir:

**Pane 9 (proxy, clean compact logging):**
```
**Pane 9 (proxy, full trace logging):**
```
mcp_wezterm_send_text(pane_id=9, text="pkill -9 -f 'cargo.watch\|target.*aphrodite' 2>/dev/null\n")
sleep 2
mcp_wezterm_get_buffer(pane_id=9, lines=3)  # verify clean shell prompt
mcp_wezterm_send_text(pane_id=9, text="APHRODITE_API_KEY=*** APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'\n")
```

**Pane 8 (Hermes test, full plugin logging):**
```
mcp_wezterm_send_text(pane_id=8, text="/quit\n")  # exit Hermes
sleep 2
mcp_wezterm_get_buffer(pane_id=8, lines=3)  # verify "Goodbye!" + shell prompt
mcp_wezterm_send_text(pane_id=8, text="APHRODITE_DEBUG=1 hermes --provider custom:aphrodite-token\n")
```

**Critical**: 
- NEVER use mcp_wezterm_send_text with \\x03 — WezTerm sends literal text, not Ctrl+C bytes. Always use `terminal(command=\"pkill -9 ...\")`.
- Plugin changes require Hermes restart (plugin is cached in memory at session start).
- RUST_LOG=aphrodite=info shows aphrodite logs only (no hyper/rustls noise).
- **Verify after every MCP action**: After `mcp_wezterm_send_text`, call `mcp_wezterm_get_buffer(pane_id, lines=5)` to confirm. This is how a human works — type, look, verify. Never assume text landed on a clean shell.
- **Never paste onto running process**: If cargo watch/Hermes is running, text pastes into stdin, not shell. Pkill first, verify with buffer, then send.
- **Plugin reload**: Hermes caches plugin at session start. Changes require full Hermes restart (`/quit` + relaunch). `hermes plugins disable/enable` only affects next session, not current.

## Version Bump Checklist

Must update ALL 4 locations in sync. Run after build + test pass:

| # | File | Key | Example |
|---|------|-----|---------|
| 1 | `plugins/aphrodite/__init__.py` | `BIN_VERSION` | `"v0.5.52"` |
| 2 | `plugins/aphrodite/__init__.py` | `PLUGIN_VERSION` | `"1.61.0"` |
| 3 | `plugins/aphrodite/plugin.yaml` | `version:` | `1.61.0` |
| 4 | `crates/aphrodite/Cargo.toml` | `version =` | `"0.5.52"` |

The Rust binary embeds its version at compile time — rebuild after bumping Cargo.toml. Always copy binary before tag push: `cargo build --release -p aphrodite && cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite && git tag -f -m "..." vX.Y.Z && git push aphrodite vX.Y.Z --force`.

## Core Architecture Patterns

### Content-Addressable Store ("Pop the API")
Every put is a search. Before calling the proxy, compute content hash locally:
```python
h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
if h in _inline_store:
    return {"hash": h, "source": "cache"}  # cache hit — no API call
```
If miss → call proxy → mirror to `_inline_store`. Proxy failure → fall back to inline storage.

### Bi-Directional Store
Every operation must feed the search index:
- **Compress** → `_inline_store` + `_recent_markers` → searchable
- **Retrieve** → `_inline_store` + `_recent_markers` → searchable
- **Search** → scans all three stores, returns actionable results with retrieve hint
- Full loop: compress → search → retrieve → cache → search again finds everything

### Feature Toggle Pattern
Disruptive features (context engine) opt-in via env var:
## Coding Deep-Tune Patterns

When the user says "more" or "deeply fine tune for coding", apply these patterns:

### Content-Addressable Store ("Pop the API")
Every `put` (compress) is a `search` first. Check local cache before calling the API. Same content = same hash = no round-trip.
- In `_compress_handler`: compute SHA256 hash locally, check `_inline_store` first
- In `compress_chat_completion` (Rust): `ccr.get(&hash)` before `ccr.put()`
- Cache hit saves latency + proxy load. Fallback on miss: call proxy, store result

### Bi-Directional Index
Every operation feeds every other operation. The full loop:
```
compress → _inline_store + _recent_markers → searchable
retrieve → _inline_store + _recent_markers → searchable
search   → returns hashes → ready for direct retrieve
```
## Engine Debugging (v0.5.47-0.5.49 discovery)

The context engine was silently broken for many versions. `should_compress()` relied on `update_from_response()` being called by Hermes to provide token counts — but Hermes never calls it. The fix:

```python
tokens = prompt_tokens or self.last_prompt_tokens or (self.context_length or 1000000)
```

Three-tier fallback: Hermes-provided → API-reported → assume full context. Without this, the engine never compresses.

**Aggressive engine test**: 
```bash
APHRODITE_CONTEXT_ENGINE=1 APHRODITE_ENGINE_THRESHOLD_PCT=1 APHRODITE_ENGINE_MIN_MSGS=1 \
APHRODITE_ENGINE_PROTECT_FIRST=0 APHRODITE_ENGINE_PROTECT_LAST=0 hermes ...
```

With protect=0/0 and threshold=1%, engine compresses ALL middle messages. LLM sees: system + first msg + catalog + last msg. Useful for reducing context in long sessions, dead weight for short ones.

## Essential Tools Skip Guard

When modifying `_transform_tool_result` skip list, NEVER compress these:
```
_ESSENTIAL_TOOLS = {"skill_view", "skills_list", "skill_manage", "memory", "session_search", "read_file", "read_terminal"}
```

The agent needs immediate access to skill docs, memory, and session history without CCR retrieval roundtrips. Compressing a 56KB skill doc forces the agent to retrieve it on every use — defeating the purpose.

## Coding Deep-Tune Patterns

When the user says "more" or "deeply fine tune for coding", apply these patterns:

### Bi-Directional Store
Every operation feeds the search index:
- **Compress** → stores hash:content in `_inline_store` + metadata in `_recent_markers`
- **Retrieve** → caches content in `_inline_store` + tracks in `_recent_markers`
- **Search** → scans `_inline_store`, `_conv_index`, `_recent_markers` — returns actionable results

### Content-Addressable ("Pop the API")
Before calling the proxy, check local cache first:
```python
h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
if h in _inline_store:
    return {cache hit — no API call}
# Only hit proxy on miss
```

### Adaptive Thresholds
```python
threshold_for(ct) × base: error=8×, code=4×, diff=2×, default=1×, logs=÷2, linter=÷2
```
Auto-tune via compression_ratio_ema — high ratios raise thresholds, low ratios lower them.

### Rust Borrow Checker Pattern
When adding state calls after value mutation, the reference from `as_str()` or `as_array_mut()` is still alive. Fix: copy to owned String or wrap in block to drop borrow:
```rust
let (compressed, orig_len) = {
    let ct = detect_content_type(content);
    let compressed = smart_marker(&hash, content, ct);
    let len = content.len();   // borrows content
    state.record_compression(ct);
    (compressed, len)          // move owned values out, drop borrow
};
*content_val = Value::String(compressed);  // now safe to mutate
```

### Honest Self-Assessment
When the user challenges your work, don't defend. Create an honest-gaps assessment:
- Go through every task/plan item
- Mark what was actually done vs skipped
- For each skip, explain WHY (with research, not hand-waving)
- Categorize by effort/impact for prioritization
- Save to `.hermes/plans/honest-gaps.md`
- See `references/honest-assessment.md` for the full methodology and common red flags.
3. Add to `provides_tools` in `plugin.yaml`
4. Add to `install_message` in `plugin.yaml`
5. Update docstring tool count in `__init__.py`

## Rules

- Patch + write_file for all edits — never sed/awk/python
- `cargo build --release` AND `cargo test -p aphrodite` must both pass before bumping
- Commit inside vendor/headroom submodule first, then `git add --force vendor/headroom` + commit parent
- Each iteration is one focused change + release
- aphrodite.toml: never commit dev=true or API keys
- CCR markers: `<<<CCR:hash|type|size>>>` ASCII only
- When user says "more": keep iterating, don't summarize. Architectural changes welcome.
- Hermes built-in compression MUST be disabled: `hermes config set compression.enabled false`
- Context engine is opt-in via `APHRODITE_CONTEXT_ENGINE=1`, `context.engine: default` otherwise
- `agent.resume_session: false` and `agent.tui_auto_resume_recent: false` for clean TUI launches

## Pitfalls

- **Context compression runaway**: If user reports constant "compacting context", check: `compression.enabled` in Hermes config (must be false), `ENGINE_THRESHOLD_PCT` (0=disabled, 50=normal), `ENGINE_MIN_MSGS` (30). Old default was 0=always.
- **Tag force-push**: Always use `git tag -f -m "message" vX.Y.Z` (annotated tag required for -f).
- **Release binary timing**: Copy binary BEFORE tag push. Sequence: copy → tag → push tag.
- **Rust borrow checker**: When adding state calls after value mutation, clone the reference to an owned String first, or wrap in a block `{ let x = compute(); (result, len) }`.
- **replace_all=true**: Can corrupt Rust test structs. Prefer single-match patches. Always run `cargo test -p aphrodite` after.
- **Secret newtype**: `pub struct Secret(pub(crate) String)` with `From<&str>` and `From<String>` impls.
- **dirs crate**: Add `dirs = "5"` to Cargo.toml. Use `dirs::data_dir().join("aphrodite").join("ccr.db")`.
- CRLF warnings from git are cosmetic on macOS — ignore.
- **Env passthrough**: When launching background proxies, Hermes doesn't inherit env vars to subprocesses. Set `terminal.env_passthrough: ["APHRODITE_API_KEY","PATH","HOME"]` in config.yaml. See `hermes-plugin-development` skill for full details. Without this, proxies get 401 because they never saw the API key.
- **Provider auth mismatch**: When adding a new Hermes provider that routes through a proxy, use the SAME `api_key_env` value as the working default provider (e.g., `APHRODITE_API_KEY`). Do NOT create a new env var name like `HEADROOM_DEEPSEEK_KEY` — it may have a different or empty value, causing 401 loops. The provider config's `provider:` field (e.g., `openai` vs `deepseek` vs `custom`) controls how Hermes constructs the request; the `api_key_env` controls which env var Hermes reads for the Bearer token.
- **Engine silently dead**: See `references/engine-silently-dead.md` for root cause (update_from_response not called) and triple-fallback fix.
- **MCP verification rule**: See `references/mcp-verification-rule.md` — never paste onto running process, always verify with get_buffer before and after.
- **CCR ghost entries**: See `references/catalog-display-fixes.md` for three-layer defense against empty hashes, abc123 placeholders, and dedup.
- **Rust field deletion**: Removing struct fields (notify_url, request_history) cascades to 10+ edit sites. Patch tool is too fragile for this — revert and plan carefully. Prefer write_file for the full struct or do one site at a time with a build between each.
- **CCR:{} empty hash in catalog**: When catalog shows `CCR:{} | 0B` entries, hash is empty dict/string from bad parse. Fix with `str()` cast + `len(h) < 4` filter at parse level, `continue` skip at display level. Both guards needed — see `aphrodite-dev-workflow` references/ccr-empty-hash-bug.md for full fix pattern.
- **Preview extraction with `]` bracket**: `_extract_preview` was written for old `[CCR:...]` format — split on `]` which doesn't exist in `<<<CCR:...>>>`. Fix: split on `>>>` instead. `after.split('>>>', 1)[-1].strip()`.
- **_parse_ccr_markers regex**: Use `re.finditer()` not `re.findall()`. `findall` returns captured groups only (no position). `finditer` gives match objects with `match.end()` for correct preview extraction position.
- **Bi-directional store**: Every compress/retrieve must also store in `_inline_store` and track in `_recent_markers`. Content-addressable: same content = same hash = cache hit = no API call.
