# Master-Worker Pattern for Hermes -z

## Definition

**Master-worker**: Master decomposes, dispatches, composes. Workers execute. Master never touches the work. Fully general — applies to any domain, any toolset.

Alternative names: fan-out/fan-in orchestration, task delegation pattern, async agent swarm, reasoning-execution separation.

## Architecture

```
User → Master (v4-pro) → Workers (flash × N) → Master → User
         │                      │
     ONLY:                 ONLY:
  terminal(bg=true)     read → patch → verify → report
  process(poll)          NEVER sub-flash, NEVER sub-poll
```

## Session Data (2026-06-16)

| Metric | Value |
|--------|-------|
| Flash sessions | 19 |
| Pro sessions | 1 (orchestrator) |
| Total tool calls | ~480 |
| Max concurrency | 4 at peak (09:25) |
| Avg tool calls/agent | ~24 |
| Avg session duration | 8-12 min |
| Bugs fixed | ~130 across 12 audit waves |
| Releases | v0.5.57 → v0.5.61 |
| Retries | 2 guardrail hits, 2 shell quoting clashes |

## Tool Call Distribution

| Tool | % of total |
|------|-----------|
| read_file | 25% |
| patch | 21% |
| terminal | 20% |
| search_files | 11% |
| todo | 8% |
| write_file | 6% |
| session_search | 4% |
| other | 4% |

## Agent Types

| Type | Permissions | Avg time | Example |
|------|------------|----------|---------|
| Python patch | read + patch + py_compile | 8 min | 9 hooks bugs |
| Rust patch | read + patch + cargo check | 12 min | 7 proxy.rs bugs |
| Rust feature | read + patch + cargo check + test | 18 min | Token caching |
| Analysis | READ ONLY | 10 min | Code audit |
| Commit/Release | git add/commit/push + gh release | 12 min | v0.5.61 |
| Config/Symlink | read + rm + ln + verify | 14 min | 7-profile chain |

## Concurrency Timeline

```
09:22 ┤ ═══ P0/P1 hooks ──┐
09:22 ┤ ═══ P2/P3 hooks ──┤  concurrent pair
09:25 ┤ ═══ P0 core ──────┐
09:25 ┤ ═══ P1 files ─────┤  
09:25 ┤ ═══ P2 files ─────┤  peak=4 concurrent
09:27 ┤ ═══ P0/P1 tools ──┘
09:33 ┤ ═══ N-batch P0 ───┐
09:33 ┤ ═══ N-batch P1 ───┤  concurrent pair
09:33 ┤ ═══ N-batch P2 ───┘
```

## Per-Agent Tool Calls

| Agent | Messages | Tool calls |
|-------|----------|------------|
| Token caching (Rust) | 79 | ~13 |
| P0/P1 hooks (Python) | 60 | ~13 |
| Average | ~70 | ~13 |

Pattern: read file in 500-line chunks → search for patterns → patch → verify → report.

## Pitfalls

- **Poll recursion**: Workers launching sub-workers. Stopped by skill rule.
- **Blocking**: Polling immediately after dispatch. Launch all → do other work → poll.
- **Shell quoting in hermes -z**: Double quotes and Rust format!() strings break shell. Use single quotes when instructions contain `"`, `>`, or `{`.
- **Guardrail hits**: 5 repeated failures → halt. Split into smaller batches, use different file paths.
- **Cargo check vs run**: Workers must NEVER run `cargo run` or `cargo watch`. Verify with `cargo check` only. Pane 17 handles live rebuild.
- **Kill stale processes**: `process(action="kill")` for hung tasks.
