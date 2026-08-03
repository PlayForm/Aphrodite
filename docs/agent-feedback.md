# Aphrodite Agent Feedback & Best Practices

> **Date:** 2026-07-31  •  **Scope:** Generalized feedback for agents using the Aphrodite CCR compression engine with Hermes. Adapted from session feedback collected across multiple projects (Aphrodite, STE-Code) and stripped of all user-specific paths, environment variables, or environment-specific values.

---

## 1. CCR Marker Handling - The #1 Thing Agents Get Wrong

### What a CCR marker looks like:
```
<<<CCR:hash|type|size>>>
```

- `hash` - BLAKE3 hash (SQLite primary key), 40 hex chars
- `type` - content type: `code`, `log`, `diff`, `error`, `json`, `build_output`, `text`
- `size` - byte size of the original uncompressed content

### The rules (non-negotiable):

1. **Every `<<<CCR:hash|type|size>>>` marker IS the compressed content.**
   Retrieve it immediately with `aphrodite_retrieve(hash=...)`. Do not defer,
   do not batch other tool calls first, do not try to "infer" what the content
   might be.

2. **Never re-read a file** with `read_file` when you already have a live CCR
   marker for it. The marker IS the content. Re-reading wastes an API call and
   tokens.

3. **Never treat markers as opaque.** The hash inside `<<<CCR:...>>>` is the
   key to the full original content via `aphrodite_retrieve(hash=...)`.

4. **If `aphrodite_retrieve` fails** (`found=false`): fall back to the original
   tool (`read_file`, `terminal`) for that specific hash. Do NOT skip retrieval
   entirely and hallucinate content.

5. **After EVERY tool call:** scan the output for `<<<CCR:`. If found, retrieve
   ALL markers before proceeding. This is non-negotiable.

6. **Multiple markers in a batch:** retrieve them ALL in the same turn before
   any other action. Batch the `aphrodite_retrieve` calls together.

7. **Nested markers:** after calling `aphrodite_retrieve`, the result itself
   may contain nested CCR markers. Retrieve those too - recursively, up to 5
   levels deep. `resolve::expand` handles this automatically.

### Consequences of ignoring CCR:
- You respond with "I got compressed output" instead of the actual content
- You waste turns re-reading files the engine already fetched
- The user gets low-quality responses based on missing data
- This is the #1 cause of poor agent performance in compressed sessions

### Retrieval is cheap (sub-ms from local SQLite). Thinking/acting on the content is what costs tokens.
Always retrieve first, then decide what to do with the content.

---

## 2. Engine Health & Testing Checklist

Before relying on CCR tools in a session:

1. **Check engine health:**
   ```python
   aphrodite_stats()
   # Must show: engine_enabled=true,
   # proxies.token.alive=true, proxies.cache.alive=true
   ```

2. **Run the smoke test:**
   ```python
   aphrodite_test(mode="quick")
   # Must return: status="ok", passed=1, total=1
   ```

3. **Verify end-to-end roundtrip:**
   ```python
   result = aphrodite_compress(content="Test content\nHello world", type="text")
   hash = result["hash"]
   aphrodite_retrieve(hash=hash)  # Must return found=true
   ```

---

## 3. The 13 Aphrodite Tools (Quick Reference)

| Tool | Purpose | Key Parameters |
|---|---|---|
| `aphrodite_stats` | Check health, version, thresholds, proxy status | none |
| `aphrodite_test` | Smoke test: compress→retrieve→search roundtrip | `mode` (quick/default) |
| `aphrodite_compress` | Compress content into CCR | `content` (req), `type` (code/log/diff/error/json/build_output/text) |
| `aphrodite_retrieve` | **Retrieve original content from CCR** | `hash` (req), `query` (opt filter), `path` (opt file bypass) |
| `aphrodite_search` | Search CCR entries | `query` (req), `type` (opt filter) |
| `aphrodite_catalog` | List all CCR entries | `mode` ("toc" for compact, default full) |
| `aphrodite_diff` | Show conversation turn history | none |
| `aphrodite_rebuild` | Report binary version + proxy health | none |
| `aphrodite_reclassify` | Retroactively classify/metadata-enrich entries | `hash` (opt, omit for all) |
| `aphrodite_prefetch` | Read files in background → compress to CCR | `paths` (array of file paths) |
| `aphrodite_prefetch_status` | Live prefetch schedule | none |
| `aphrodite_files` | List all file paths referenced in session | none |
| `aphrodite_directive` | Manage behavioral directives | `action`, `name` |

---

## 4. Auto-Expand vs. Manual Retrieval

With `auto_expand = true` (the default in shipped configs):
- **Tool outputs** are auto-expanded inline - you see the full content, no markers
- **Raw terminal/proxy output** may still produce CCR markers - retrieve with
  `aphrodite_retrieve(hash)`

Even with auto-expand, when you SEE a `<<<CCR:hash|type|size>>>` marker in any
output, **retrieve it immediately**. Auto-expand handles tool results, but
markers can appear from:
- Direct terminal proxy output
- Background worker results returned as logs
- Compressed context from other agents/sessions
- Catalog/recall summaries

---

## 5. Prefetch - Anticipate the Next Turn

Prefetching is about loading files you **WILL need next turn**. Retrieval is
about loading files you need **NOW**.

- **After `search_files`:** immediately prefetch the top 5-10 results before
  reading them one by one.
- **After reading a file:** identify what it imports/references. Prefetch those.
  The engine loads them while you process the current file.
- **After an edit:** run the relevant test AND prefetch the test output file.
- **When approaching a new directory:** prefetch its key files (config, main
  entry point, README).
- **Use `aphrodite_prefetch` for any batch of 3+ files.** A single prefetch
  call is cheaper than 3 sequential reads.
- **If you prefetch a file and get a CCR marker back, retrieve that marker
  IMMEDIATELY** - don't wait for "next turn."

---

## 6. Background Process Management

**Use `process(action='poll')`, never `wait` or blocking timeouts.**

- `process(action='wait')` blocks the conversation for up to 60-300 seconds,
  freezing it and preventing mid-turn user messages.
- `process(action='poll')` is non-blocking and returns immediately with
  current status.

**Best practice:**
- Launch long tasks with `terminal(background=true, notify_on_complete=true)`
- Check status with `process(action='poll', session_id=...)`
- Use `notify_on_complete=true` to get automatic completion notification,
  reducing the need for active polling
- Terminal commands with `sleep` loops in `timeout` mode also block. Use
  short `sleep` loops inside `background=true` processes instead

---

## 7. API Parallelism Limits

**Maximum 2-way concurrent LLM API calls.**

- 5-way parallel batches cause severe API contention - workers take 2-3x
  longer and fail frequently (timeouts, empty outputs, 502s)
- 2-way parallel maintains normal worker completion times (~55-90s each)
- Use sequential chains for multi-batch runs: launch multiple chain
  processes, each processing batches sequentially. E.g., Chain A:
  batches 11→12→13→14, Chain B: batches 15→16→17→18
- When multiple workers hit the same LLM API concurrently, rate limiting
  kicks in. Sequential worker execution within a batch is optimal for
  single-batch runs; 2-chain parallelism for multi-batch
- Low CPU = API throttling: if processes show <5s CPU time over 2+ minutes
  of wall time, the API is rate-limiting. Kill and restart with less
  concurrency

---

## 8. File Editing Best Practices

**Never use `sed` for file edits.** `sed` on structured files causes:
- Doubled comments (`# tag` → `# tag # tag`)
- Tab/space indentation corruption in template strings
- Subtle regex corruption that's hard to detect

**Correct approach:**
- Use the `patch` tool for targeted find-and-replace edits
- Use `write_file` for complete file rewrites
- For bulk operations, use a proper Node.js/Python script saved to a
  dedicated maintenance directory

**When `patch` fails 3 times on the same region:** stop patching and use
`write_file` with the full corrected content. Fuzzy matching can duplicate
content, corrupt indentation, and insert fragments at wrong locations.

---

## 9. Git Workflow for Generated Files

**Gitignore negation is required** for generated output files:
- If `.gitignore` contains a directory pattern (e.g., `output/`), files
  inside that directory cannot be `git add`-ed, even with explicit paths
- Add a negation rule (`!output/`) to `.gitignore` first, then use
  `git add -A` to stage all files including previously-ignored ones
- **Manual commit as fallback:** when a script's internal `git commit` fails
  due to `.gitignore`, run `git add -A path/to/files && git commit -m "..."`
  manually - the script's `git add` + `git commit` calls fail silently on
  ignored files

---

## 10. Directives - Behavioral Context Injection

Aphrodite ships five built-in directives baked into the binary via
`include_str!`:

| Directive | Behavior |
|---|---|
| `focus` | Stay targeted: at most 1-2 tools per turn, prefer `aphrodite_retrieve` over re-reading files |
| `foresight` | Anticipate I/O, prefetch files you'll need next turn. After search_files, prefetch top 5-10 results |
| `ccr-handling` | CCR marker handling rules - the core retrieval discipline from sections 1-2 above |
| `cleanup` | Summarize and prune: progress summary every 5 turns, catalog sweeps, verify nothing left behind |
| `explore` | Read broadly: 2-3 related files per turn, prefetch batches of related paths |

### Discovery and loading

| Rule | Behavior |
|---|---|
| Search order | `./directives/` (working directory), then `~/.hermes/directives/` - first directory that exists wins; they are NOT merged |
| File filter | Only `*.md` files; anything else is silently skipped |
| Naming | Directive name = file stem (`focus.md` → `focus`) |
| Per-file cap | 2,000 chars per directive body (char-safe truncation, `…` appended) |
| Combined cap | 4,000 chars across all active directives' injected text combined |
| Load condition | Directories load **unconditionally** when present - loading is not gated on `[directives] active` being non-empty |
| Built-in fallback | When no `directives/` directory exists, the 5 baked-in directives are loaded automatically |
| Active default | When `[directives] active` is empty and no disk directives found, `focus` + `foresight` are seeded as active |

### Runtime management

Use `aphrodite_directive` to manage directives at runtime:
```python
aphrodite_directive(action="list")                    # list active/available
aphrodite_directive(action="swap", name="explore")    # replace active set
aphrodite_directive(action="add", name="ccr-handling")  # append to active set
aphrodite_directive(action="load", name="focus")     # activate on demand (errors on unknown)
aphrodite_directive(action="remove", name="cleanup")   # drop from active set
aphrodite_directive(action="reset")                    # clear active set
```

- The active set persists across a session reset - it's per-process state
- Unknown directive names are filtered out (not errors)
- A `swap`/`add`/`remove`/`reset` latches `manual_directive_turn` to suppress
  phase-aware auto-swaps until the user explicitly re-enables them

### Injection mechanics

`pre_llm_call` builds the directive block via `build_directive_context` and
appends it to the per-turn injected context:

```text
[directives: focus, foresight]
focus:
  focus - stay targeted, minimal tool usage
  Each turn: use at most 1-2 tools. Prefer retrieval over re-reading.
  One primary action per turn
  Use aphrodite_retrieve(hash) for any <<<CCR:hash...>>> you see
```

| Detail | Behavior |
|---|---|
| Header line | `[directives: name1, name2]` - active names, comma-joined |
| Body | Each active directive's **full** (per-file-capped) body, not just its title line - leading `#` markers stripped, blank lines dropped, remaining lines indented |
| Placement | Appended after the catalog summary in the hook's returned context string; empty when no directives are active |
| Frequency | Every `pre_llm_call` - the block reflects the active set at that moment |

---

## 11. First-Turn Session Injection

On the **first turn only** (`turn_counter == 0`), the engine injects a
one-shot orientation block that explains how the compression system works.
This is built from the `[prompts] session_inject` key in `aphrodite.toml`:

```toml
[prompts]
session_inject = """
[APHRODITE] v{VERSION} active.
  This session is running with CCR compression. Tool outputs larger than a
  few hundred bytes are replaced with markers like <<<CCR:hash|type|size>>>.
  The marker IS the content - retrieve it before acting on it:
  aphrodite_retrieve(hash) → full original content (sub-ms, local).
  After EVERY tool call: scan for <<<CCR: and retrieve ALL markers first.
  NEVER re-read a file you already have a marker for. Use aphrodite_catalog
  to see stored entries, aphrodite_prefetch for background file loads, and
  aphrodite_directive("list") for active behavioral directives.
  Layer 2: per-turn catalog injected below each turn.
  Layer 3: load the aphrodite-tool-guide skill for full tool reference.
"""
```

- `{VERSION}` is replaced with the compiled-in `CARGO_PKG_VERSION`
- Set to `""` to disable entirely
- When the key is absent, a compiled-in default (`SHIPPED_SESSION_INJECT`)
  is used so even a minimal config gets orientation
- The block has the **highest survival priority** in the per-turn context
  assembler - it always reaches the model on turn 0, then vanishes forever

---

## 12. CCR Lifecycle & Thresholds (Reference)

**Storage tiers** (by content size and mode):

| Tier | Threshold | Backend | Capacity | TTL |
|---|---|---|---|---|
| Inline | < 256B | `LruCache<String, String>` | 1,024 entries | LRU eviction only |
| Cache mode | > 8KB | `InMemoryCcrStore` (DashMap) | 10,000 entries | Configurable (default 3600s) |
| Token mode | > 1KB | `SqliteCcrStore` (SQLite) | Unlimited (disk) | Configurable (default 3600s) |

**Per-type multipliers** (token mode, 1KB base):

| Type | Multiplier | Effective threshold |
|---|---|---|
| error | ×8 | 8,192 |
| code | ×4 (default) | 4,096 |
| diff, git, text | ×2 | 2,048 |
| tool_output, json | ×1 | 1,024 |
| linter, build_output, log | ×1 (BASE, not halved) | 1,024 |

> **Correction:** `linter`, `build_output`, and `log` are pinned at BASE
> threshold, not halved. `proxy.rs::threshold_for` returns `base` for these
> three types immediately - before the auto-tune multiplier is applied.

---

## 13. Proxy Architecture

- **Token proxy** (`:9798`) - token-level compression, requires API key for
  management endpoints
- **Cache proxy** (`:9797`) - cache-mode compression, management endpoints
  accept any loopback caller (no credential needed)
- Both proxies share the same CCR database at `$HOME/.hermes/aphrodite/ccr.db`
  (SQLite)
- Binary: `$HOME/.hermes/aphrodite/binaries/aphrodite` (auto-updated)
- Dylib hot-reloads on file modification - rebuild Rust code and the plugin
  picks it up without restart

---

## 14. Quality Gate Enforcement

**Commentary detection is valid.** Quality gates that check for meta-commentary
phrases (e.g., "here is", "in summary", "let me check") in the last N lines of
output are valid checks. Rely on the retry mechanism (up to 3 attempts) rather
than weakening the gate.

**Triple blank lines in raw extraction output are acceptable** at the
extraction stage - they will be addressed during refinement.

---

## Related Resources

- **Built-in directives:** `crates/aphrodite/src/builtin_directives/` (baked into binary via `include_str!`)
- **Plugin hooks:** `docs/plugin/hooks.md` - the `pre_llm_call` lifecycle
- **Directive reference:** `docs/plugin/directives.md` - full directive system documentation
- **CCR lifecycle:** `docs/ccr/lifecycle.md` - compression, caching, retrieval, expiry
- **Tool reference:** `docs/tool-relay/tools.md` - full 13-tool reference
- **Config schema:** `docs/config/aphrodite-toml.md` - the `[prompts]` and `[directives]` sections
