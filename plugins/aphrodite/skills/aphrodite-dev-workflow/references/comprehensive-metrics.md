# Comprehensive Metrics Collection

Full-session metrics dump — every data source, how to pull it, and how to interpret it.

## Data Sources (ordered by depth)

### 1. aphrodite_stats (Python tool)
Quick health + engine overview. Returns proxy alive/dead, CCR counts, tokens saved,
engine threshold status. The `ccr_entries` field shows `"?"` because the Python plugin
doesn't have a proxy entry count — use the raw /stats endpoint for that.

### 2. aphrodite_catalog (Python tool)  
Session-level CCR items: hashes, types, sizes, previews, conversation turns, files referenced.
Only shows entries created THIS session. Does NOT include proxy-only CCR entries.

### 3. Raw aphrodite proxy /stats endpoint
```bash
curl -s http://127.0.0.1:9798/stats
curl -s http://127.0.0.1:9797/stats
```
Returns full JSON with: request history (last 50, with method/path/status/elapsed),
latency buckets (microsecond), compression EMA, CCR created/hits/misses, cache hits/misses,
tool relay calls, tokens saved, mode, version, error log.

### 4. Headroom proxy (:9799) — the layer above
The headroom caching proxy sits BETWEEN Hermes and aphrodite. Its stats are critical
for understanding the full system. Two key endpoints:

**Health + runtime:**
```bash
curl -s http://127.0.0.1:9799/health
```
Returns: service, version, uptime_seconds, all checks (startup, http_client, cache,
rate_limiter, upstream), runtime config (concurrency, compression executor workers),
config flags (optimize, cache, rate_limit, memory, ccr_marker).

**Full stats (large JSON, often CCR-compressed):**
```bash
curl -s http://127.0.0.1:9799/stats
```
Returns: request volume (by method/path/status/model), tokens input/output/saved,
latency (avg/min/max), overhead, TTFB, cache entries/hits, compression stats,
persistent savings, prefix cache stats, TOIN patterns, recent requests (last 10),
proxy inbound breakdown.

**Prometheus metrics:**
```bash
curl -s http://127.0.0.1:9799/metrics
```

**Key headroom signals:**
- `optimize: false` — headroom is in pure passthrough, no compression
- `ccr_entries: 0` — no CCR at headroom layer (all happens at aphrodite)
- `tokens_saved (session): 0` — expected when `--no-optimize` is set
- `persistent_savings.lifetime.tokens_saved` — the REAL number (survives restarts)
- `persistent_savings.lifetime.requests` — total requests since install
- `persistent_savings.savings_history` — time-series of cumulative savings
- `proxy_inbound.by_path` — shows Ollama model-discovery 404s (useless round-trips)

### 5. Persistent savings file (~/.headroom/proxy_savings.json)
Headroom's lifetime stats survive proxy restarts. Read directly:
```bash
python3 -c "import json; d=json.load(open('$HOME/.headroom/proxy_savings.json')); print(f'{d[\"lifetime\"][\"requests\"]} reqs, {d[\"lifetime\"][\"tokens_saved\"]:,} tokens saved, {d[\"lifetime\"][\"total_input_tokens\"]:,} total input')"
```
This is the ONLY persistent counter in the entire stack. Aphrodite proxy counters
and Python inline stores are all in-memory and reset on restart.

### 6. Process tree (ps aux)
```bash
ps aux | grep -E 'aphrodite|headroom|cargo.watch|hermes' | grep -v grep
```
Shows: which proxies are running, how long, CPU usage, worker count (headroom
multiprocessing), cargo-watch auto-reload, Hermes session PID. Critical for
understanding the runtime topology — who routes through whom.

### 7. Proxy /metrics endpoints (Prometheus, both layers)
```bash
curl -s http://127.0.0.1:9798/metrics   # aphrodite
curl -s http://127.0.0.1:9799/metrics   # headroom
```

### 8. .hermes/MASTER-TASKS.md
Bug audit with scorecard: severity breakdown (🔴/🟠/🟡/🟢), per-bug status,
release history, commit timeline.

### 9. .hermes/build-status.json
Last build timestamp, status (ok/building/error), version, errors array.

### 10. aphrodite_files (Python tool)
All file paths referenced this session, grouped by tool type.

### 11. Hermes config (providers section)
```bash
grep -A6 'aphrodite-cache\|aphrodite-token\|headroom-cache\|headroom-token' ~/.hermes/config.yaml
```
Shows the full provider chain: base_url (which port), api_key_env, max_tokens, provider type.

## Common Metrics Interpretation

### EMA compression ratio stuck at 10.00
The EMA initializes at 10.00 and only updates when compression actually triggers.
If `requests_compressed` is 0 and `compressions_by_type` is empty, the EMA never moved
— no request crossed the compression threshold.

### Cache proxy (:9797) all zeros
The Python plugin routes all traffic to token proxy (:9798) first. Cache proxy
only gets traffic when the token proxy is dead — a fallback, not a primary.

### Engine compressions = 0
The context engine fires when prompt_tokens >= threshold_tokens.
With 1M context and 50% threshold, that's 500K tokens. At 50K tokens (5%),
the engine won't fire for many turns.

### Headroom session savings = 0 but lifetime savings = 33M+
The headroom proxy is running with `--no-optimize` flag. Every request passes through
untouched. Session counters show zero compression. But the persistent savings file
(`~/.headroom/proxy_savings.json`) shows cumulative lifetime stats from previous
sessions when optimization was ON. The lifetime number is the real savings history;
the session number is current-passthrough reality.

### Headroom passthrough detection
If headroom `/stats` shows all of: `requests_compressed: 0`, `tokens_saved: 0`,
`ccr_entries: 0`, `overhead_ms: 0`, `ttfb_ms: 0` — the proxy is in pure passthrough
(launched with `--no-optimize --no-ccr-marker`). It's a transparent relay.
All compression savings come from the next layer (aphrodite).

### Ollama model-discovery 404s
Headroom `proxy_inbound.by_path` shows 5 × 4 = 20+ 404s per session from `/api/v1/models`,
`/api/tags`, `/v1/props`, `/props` — these are Ollama-style model discovery probes
that hit the catch-all route. Harmless but wasteful. Hermes probes for Ollama even
when using a custom provider.

## CCR Persistence: NONE

All stores are in-memory:
- Token proxy (:9798): in-memory HashMap (despite the "token" name)
- Cache proxy (:9797): in-memory HashMap (>8KB threshold)
- Python inline store: collections.OrderedDict in-process
- Catalog: Python list, session-scoped

Nothing survives proxy restart or Hermes exit. The catalog is empty on fresh sessions.
Cross-session context is in Hermes' state.db, not the CCR cache.

## Pull Order for "total metrics"

```bash
# 1. Python tools + headroom health (fire in parallel)
aphrodite_stats()
aphrodite_catalog()
aphrodite_files()
curl -s http://127.0.0.1:9799/health           # headroom layer

# 2. Raw proxy stats (all three ports)
curl -s http://127.0.0.1:9798/stats             # aphrodite token
curl -s http://127.0.0.1:9797/stats             # aphrodite cache
curl -s http://127.0.0.1:9799/stats             # headroom (large, often CCR-compressed)

# 3. Process tree + persistent savings
ps aux | grep -E 'aphrodite|headroom|cargo.watch|hermes' | grep -v grep
python3 -c "import json; d=json.load(open('$HOME/.headroom/proxy_savings.json')); print(f'lifetime: {d[\"lifetime\"][\"requests\"]} reqs, {d[\"lifetime\"][\"tokens_saved\"]:,} saved')"

# 4. File-based
read_file(".hermes/MASTER-TASKS.md")            # bug scorecard
read_file(".hermes/build-status.json")          # build health
grep -A6 'aphrodite-cache\|aphrodite-token\|headroom-cache' ~/.hermes/config.yaml  # provider chain
```

## "more?" Escalation Pattern

When the user says "more?" after a metrics dump, they are asking for the NEXT data source
layer — not wider coverage of the same layer. This is a depth-first escalation, not
breadth-first. Follow this sequence:

```
Layer 0: aphrodite_stats + aphrodite_catalog          (Python tool view)
Layer 1: curl :9798/stats + :9799/health               (raw proxy endpoints)
Layer 2: ps aux | grep aphrodite\|headroom              (process tree, uptime, CPU)
Layer 3: aphrodite.toml + ~/.hermes/config.yaml         (config, provider chain)
Layer 4: wc -l + git log --oneline                      (codebase size, commit history)
Layer 5: ~/.headroom/proxy_savings.json                 (persistent lifetime stats)
Layer 6: .hermes/MASTER-TASKS.md                        (bug audit scorecard)
```

Each "more?" = advance one layer. Do NOT repeat the same layer. Do NOT summarize across
layers — the user wants raw data from the next source, not a digest of what they already
saw. If you reach the last layer and the user says "more?" again, say so explicitly:
"Layer 6 reached — that's every data source in the system."

Typical session flow: benchmark pipeline → stats → "more?" → raw proxy → "more?" →
process tree + config → "more?" → codebase + git → "more?" → lifetime savings →
"more?" → bug audit. Six escalations from initial benchmark to full system audit.

### Provider Chain Discovery

To understand the full request path, trace the provider chain:

```
hermes config: model.provider → base_url → which proxy
proxy /health: upstream URL → which API backend
ps aux:       which processes are actually running
```

Common chain: `hermes → headroom-cache (:9799) → aphrodite-token (:9798) → DeepSeek API`
If headroom shows `optimize: false` and `ccr_entries: 0`, it's in pure passthrough —
all compression happens at the aphrodite layer.
