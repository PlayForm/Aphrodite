# Bench Session 2026-09-23 - live benchmarking foundation

**Status:** session record - the live-benchmark harness work, runs, findings,
bugs found, and the gaps to advance. Companion to
`bench/conversational/BENCHMARK-METHODOLOGY.md` (the formal spec).

## What was built (all under bench/conversational/)

### Atomization (per the crates/ convention)
`harness.py` / `live_runner.py` / `conversations.py` / `visualize.py` were
atomized into singleton-module packages - one function per file, nameless
re-exports, reverse-hierarchical:

- `harness/` - provider.py, scenarios.py, results.py, proxy_manager.py,
  provider_client.py, proxy_client.py, tokens.py, runner.py, binary.py,
  run.py, paths.py (RESULTS_DIR - all writes constrained to results/),
  `__init__.py`, `__main__.py`
- `live/` - task_prompt.py, agent.py, scenario.py, stats.py, cli.py
- `fixtures/` - types.py + one module per conversation
- `visualize/` - one module per chart
- Flat files kept as thin shims (Maintain/scripts/bench/benchmark-eval.py
  does `import conversations`)

### Live runner (real agent sessions via Hermes' own AIAgent)
`python3 -m live` triggers real AIAgent sessions (Hermes' agent API - a
programmatic normal session, as a user would run) against the same task
fixtures. Per-cell artifacts, all under results/:

```
results/<run>/<variant>/<task>/
├── workbench/       ← pre-staged isolated copy of the task workspace
├── hermes-home/     ← per-cell isolated HERMES_HOME (variant-scoped)
├── stream.jsonl     ← live text deltas (written as they arrive)
├── trajectory.json  ← full message trace (tool calls, responses, markers)
└── result.json      ← summary + task + activity + tokens + containment
```

## The 3 configuration circles (the granular A/B/C)

Same task, same model, three aphrodite configurations - the unit of
comparison:

| variant | plugin | proxies | meaning |
|---|---|---|---|
| `full` | yes (symlinked) | cache+token | CCR via proxy path |
| `baseline` | yes (symlinked) | none | inline CCR only |
| `off` | **empty plugins dir** | none | NO aphrodite - true control |

Run: `python3 -m live --variants full,baseline,off --conversation coding_task`

Isolation: per-cell HERMES_HOME staged under results/ - empty plugins/ for
off, plugin symlinked + runtime copied for full/baseline; config.yaml + .env
are READ-ONLY symlinks to the real home (zero local-config mutation).

## Runs & findings

### glm-contained-1 (baseline, 5 tasks, GLM-5.3-Flash)
All 5 completed. Containment mostly held; 2 real read-only escapes (a /tmp
search, one sibling-results read) - both from unstaged fixtures forcing the
agent to search. Confirmed: writes were fully confined everywhere.

### glm-variants-1 (coding_task × 3 circles) - INVALID run
**Nothing compressed in ANY variant** - a staging bug: the cell home had no
`aphrodite/` runtime dir, so the plugin's `_ensure_binaries()` found nothing
and the plugin DISABLED itself (zero hooks). full/baseline silently degraded
to off. The tool outputs were large enough to compress (9.9KB/12.6KB > 4KB
threshold) - compression should have fired but couldn't.

**Lesson:** the plugin needs its binaries in HERMES_HOME/aphrodite/binaries;
a cell home without them = plugin disabled = no compression hooks. Fixed by
copying the real runtime into the cell home.

### glm-variants-2 (coding_task × 3 circles, fixed) - first valid A/B/C
| circle | msgs | elapsed | est tokens | CCR markers |
|---|---|---|---|---|
| full | 22 | 59.8s | 5,042 | **10** |
| baseline | 44 | 89.3s | 2,983 | **6** |
| off | 34 | 370.3s | 6,512 | **0** |

Compression distinction appears: 10/6 markers in full/baseline vs 0 in off.
Cost+latency signal favors compression on this task (off = 6.5k/370s vs
full = 5.0k/60s, baseline = 3.0k/89s). Single-run, high variance - not yet
conclusive.

## Bugs found & fixed

1. **Model fallback bug** - `make_agent` was passed `args.model` (None)
   instead of `args.model or MODEL` → empty model in relayed request →
   Cloudflare 500. Fixed.
2. **cwd in run_conversation** - `run_conversation()` has no cwd param; it's
   set at agent construction (`session_cwd`). Fixed by per-cell agent build.
3. **Cell-home staging** - missing aphrodite runtime → plugin disabled → no
   compression (see glm-variants-1). Fixed: copy runtime into cell home.
4. **Containment audit false positives** - relative paths (`src/foo.rs`,
   `./x`) resolve inside the workbench but were flagged; fixed by resolving
   against workbench. Residual: `execute_code` flagged unconditionally, and
   `cd` into sibling results paths flagged (still inside results/ tree).
5. **Cheapest model** - `@cf/zai-org/glm-5.3-flash` (prompt
   $0.00000015/tok, completion $0.0000005/tok) - 3× cheaper than
   deepseek-v4-flash, confirmed live.

## Environment facts

- Session dylib: Development 1.6.0 debug build (14 tools incl.
  aphrodite_debug); `aphrodite_rebuild` reports version 1.6.0, proxies down
  (inline mode). No restart needed - this session runs the Development dylib.
- Bench cells use the SAME dylib/plugin as this session (no HERMES_HOME set
  → ~/.hermes fallback) but in a SEPARATE process - load-once-per-process,
  separate inline store.
- Provider auth: CLOUDFLARE_API_TOKEN in ~/.hermes/.env (`cfut_...`); the
  harness resolves provider from config.yaml + .env.

## Gaps to advance (the real value is next)

1. **Real token usage** - currently `estimated` (message-length/4). Real
   usage lives in API response envelopes / request dumps
   (~/.hermes/sessions/request_dump_*.json). Fix `extract_usage` to read
   those → token_source becomes `response_usage`.
2. **Large-output workspaces** - coding_task outputs were small; the
   compression signal needs fixtures that produce LARGE tool outputs (big
   files, big build logs) to trigger CCR at scale.
3. **Audit precision** - execute_code should be checked for WHAT it does,
   not flagged unconditionally; `cd` into sibling results paths is within
   the bench tree.
4. **Task workspaces for the other 4 fixtures** - exploration/debugging/
   release/compression tasks point at unstaged dirs; agents correctly
   report-missing but burn turns searching.
5. **Multi-run variance** - 1 cell per circle; need N runs for p50/p90.
6. **The 5-model fanout** - deepseek-v4-flash, qwen3-30b, llama-3.3-70b,
   gpt-oss-20b, gemma-4-26b all live-probed; the fanout runner is the next
   consumer of this machinery.
7. **Retrieval-quality metrics** (methodology §3.2) - need task evidence
   maps to compute precision/recall/timing objectively.

## Related

- `bench/conversational/BENCHMARK-METHODOLOGY.md` - the formal spec
  (claims, metric families, memory policies, provenance/workflow layers,
  net-value scorecard, release gates)
- `bench/conversational/results/` - run artifacts (glm-contained-1,
  glm-variants-1 [invalid], glm-variants-2 [valid], glm-baseline-2, ...)
- `.hermes/notes/session/CONTINUE-2026-09-15.md` - prior teknium/PR notes