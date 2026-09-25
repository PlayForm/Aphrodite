---
name: agent-session-benchmarking
description: "Use when benchmarking agents/plugins with real LLM sessions."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: aphrodite
category_taxonomy: aphrodite/agent-session-benchmarking
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, benchmark, sessions, llm, harness, monitoring, audit]
        related_skills:
            [dual-state-merge-review, code-quality-improvement, aphrodite-testing-discipline]
status: active
---

# Agent Session Benchmarking

Benchmark a Hermes agent, plugin, or compression engine with REAL LLM sessions
(a prompt corpus, not dummy data), monitored by a polling harness, and audited
from the evidence. The harness launches N one-shot `hermes chat -q "<prompt>"`
sessions, polls each process every ~2s, and records per-session evidence.

## Procedure

1. **Prepare the target**: rebuild/install the artifact under test (debug build
   into the runtime home), start the proxy/daemon, verify health
   (`/health`), wire any required API key WITHOUT printing it (inline-export
   from the secrets file inside the command). Verify versions live
   (`aphrodite_rebuild`): current binary `1.6.2`, plugin `2.2.2`,
   `BINARY_VERSION` `1.6.2`. Install layout is hooks-only:
   `~/.hermes/plugins/aphrodite` holds ONLY `plugin.yaml` + `__init__.py`;
   `~/.hermes/aphrodite` holds binaries/, `aphrodite.toml`, the BINARY_VERSION
   pin, `ccr.db`, directives/, and logs/.
2. **Check the runtime config BEFORE benchmarking**: compression/product
   toggles may be deliberately disabled (see pitfalls). Back up the runtime
   config before editing it for the test.
3. **Design the corpus to force real work**: prompts must trigger LARGE tool
   outputs (ls -la, wc, find, reading big files) so the engine has something
   to compress; small outputs stay below thresholds and the run shows
   nothing. For example-generation missions, use MULTI-VARIANT prompts
   ("run ls -laR; then git diff --stat; then read <big file>; then cargo
   check | tail") so ONE session yields several response variants
   (terminal/diff/code/build) - one-variant-per-session makes a 100-variant
   mission outlast any single child's 30-min window.
4. **Run the harness** (`--sessions N`, sequential): for each session launch
   the hermes child, poll liveness + endpoint metrics every 2s, then save the
   transcript and collect evidence.
5. **Audit from the AUTHORITATIVE source, not the transcript** - see
   references/evidence-sources.md for the layering.
6. **Report**: per-session markers/types/durations, the evidence source used,
   and any config/toggle findings.

## Pitfalls

- **Know which bench path you are running - the offline simulation harness
  and the live runner answer different questions.** The offline harness
  drives conversations via direct HTTP requests against a provider and
  measures tiktoken + CCR proxy calls; it never exercises real model
  decision-making (no tool calls, no retrieve decisions), so its numbers
  are upper-bound estimates, not model behavior. To benchmark ACTUAL
  model behavior (tool calls + CCR creates + retrievals), use the live
  runner: it launches AIAgent sessions programmatically - the harness is
  operated from inside a `hermes` session and spawns each cell as a
  normal programmatic session, as a user would. Resolve the provider from
  Hermes' own config (`~/.hermes/config.yaml` model block +
  `~/.hermes/.env` API key) - never hardcode a vendor, never source a
  separate secrets file (it may be stale/commented-out); the user's rule
  is 'use the default auth provider'.
- **Billing is not context.** Never use `sessions.input_tokens +
cache_read_tokens` as the "with" side of a savings comparison - it is API
  prompt-cache accounting (~2.9x the local payload). Reconstruct per-call
  payloads from the transcript (system + active messages, markers included)
  and compare stored vs expanded tokens; report billing separately.
- **Original marker content is not persisted.** `messages.content` stores the
  marker as sent; only the marker's `size` (original bytes) survives. Token
  estimates need a calibrated chars/tok ratio plus a sensitivity band
  (3.0-4.5) - never a single point percentage.
- **Request dumps are failure samples.** `~/.hermes/sessions/request_dump_*.json`
  exist only for `max_retries_exhausted`; use them to validate marker
  presence, never to sum totals.
- **Count the retrieval tax.** `aphrodite_retrieve` results re-enter full
  content as new tool messages and cancel that block's savings; subtract
  re-entry tokens for a net figure.
- **A marker's savings apply per call, not once.** Each API call re-sends the
  accumulated history; assistant message = call boundary; accumulate deltas
  per call.
- Micro-benchmark the dylib DIRECTLY, never through a full session - a
  compress round-trip is ~50µs; session overhead (hook marshalling, JSON
  envelopes, LLM calls) swamps it. Drive `aphrodite_compress` via the
  plugin's OWN load path in a separate python process: set
  `APHRODITE_HERMES_DYLIB_PATH` to the artifact under test, then import the
  plugin and call `_load_dylib()` + `_call_json(dylib,
"aphrodite_hermes_dispatch_tool", ...)` - this runs the real init
  sequence (FFI setup, `materialize_directives`). A HAND-ROLLED ctypes
  harness that skips that setup (no argtypes, no directives init)
  SIGSEGVs on the first dispatch - never write raw ctypes probes.
- **Never swap the installed dylib mid-session to benchmark an alternate
  build** - the plugin hot-reloads on mtime change, and every reload wipes
  ALL session CCR state (existing `<<<CCR:...>>>` markers stop resolving;
  `__init__.py` warns at the reload point). Point the env override at
  `target/debug/` or `target/release/` instead; the file is read fresh per
  process, so debug and release runs never disturb the live session.
- **Debug vs release build: expect ~8x slower + ~3x larger, byte-identical
  output.** Measured on 1.5.0 (5.2KB payload, 300 ops): 47-52µs/op release
  vs 383-392µs/op debug; dylib 4.4MB vs 12.2MB, binary 12.8MB vs 41MB.
  Same input hashes to the same CCR marker in both profiles - profile
  choice changes speed and size, never correctness. A debug dylib in the
  runtime home is fine for chasing panics (line numbers, overflow checks)
  but is NOT a production install.
- The CLI transcript DISPLAY omits tool-result markers (it shows commands +
  reasoning only). Counting markers in the transcript undercounts - the
  session store (state.db `messages`) is the authoritative evidence.
- Marker regexes must strip whitespace (80-col CLI wraps markers across
  lines) and allow the `i:` inline-hash prefix and 24-40 hex hashes; filter
  out template strings (`<<<CCR:hash|type|size>>>`, `{hash}|{ct}` examples)
  with: hex hash + concrete type + numeric size.
- In-process engine compression bypasses the HTTP proxy: proxy /metrics stay
  0 while markers appear in the model context. Proxy counters only measure
  the HTTP/tool-relay path.
- `engine_threshold_pct = 100` means "effectively disable CCR compression"
    - the shipped template may carry this deliberately. Set a real threshold
      (e.g. 30) for the benchmark and restore after.
- The engine's in-process config snapshot is taken at session handshake:
  `aphrodite_stats` keeps reporting the OLD thresholds after a config
  restore until the NEXT session starts. The proxy hot-reloads on file
  change (the watcher log line proves the proxy side only); the dylib
  bridge does not. Verify a config change with a FRESH session's stats,
  never the current one.
- Hermes sessions IGNORE the process cwd: `Popen(cwd=...)` has no effect;
  the lever is the `TERMINAL_CWD` env var on the child. `AIAgent(cwd=...)`
  does set the terminal tool's working dir (session_cwd), but the FILE
  tools (read_file/write_file/patch/search_files) take absolute paths and
  are NOT cwd-restricted - an agent with file tools can still read/write
  anywhere, so `cwd` alone is not containment. Full isolation for a
  benchmark cell = dedicated profile (isolated HERMES_HOME) + a restricted
  `enabled_toolsets` list + per-cell `cwd`. Skills stay available in every
  case: they load from the skills directory, independent of cwd, and
  `skip_context_files` skips only AGENTS.md/context files, never skills.
- Background terminal spawns get a FRESH shell: session exports and `$VAR`
  expansions do not carry; inline-export secrets inside the command. The
  background `python3` may also be older (3.9) - avoid `X | None` annotations.
- `hermes chat -q` takes no `--json` flag.
- The plugin reuses a healthy proxy on the port (health probe) instead of
  spawning a second one - expect exactly one proxy process.
- The session engine's ccr.db may live on an unobserved path (dirs-data-dir
  vs runtime-home resolution) - db deltas can be 0; do not fabricate.
- When relocating a harness script between directories, re-derive the
  results-path from the NEW parents depth - a stale `parents[N]` hop silently
  writes results to a stray directory.
- Transcript files are ZERO-PADDED (`transcript-01.txt`, not `-1.txt`) -
  verification loops iterating `range(1, N+1)` silently miss every file.
- Find a run's sessions in state.db by the ID PREFIX (the id embeds the
  timestamp: `LIKE 'YYYYMMDD_HHMM%'`) - `created_at >= datetime(...)`
  comparisons fail on the stored format.
- The content-addressed store FREEZES previews: same bytes -> same hash ->
  the stored preview comes back regardless of the CURRENT preview family.
  To capture a different family's rendering (compact vs code_first vs
  balance), use FRESH corpus content (a new file, a changed diff) -
  re-running the same command returns the old stored preview and the
  variant looks like a no-op. The freeze also applies to classification
  PROBES: run each case in a FRESH process. Read the returned `type` field
  and the preview template SEPARATELY - they can disagree (the headroom
  `detect_type` feeds the label, the semantic detector feeds the preview;
  read_file of a JSON file shows a `json` preview with a `text` label). An
  explicit non-empty caller `type` hint wins over the classifier by design
  (caller-hint-wins) - pass no hint to see the classifier's own verdict.
- Capture variants by env var on the command line
  (`env APHRODITE_*=<v> hermes chat -q "..."`) when the key has an env
  override - no TOML churn, no revert discipline. When a TOML edit is
  unavoidable, revert the key to its default IMMEDIATELY after the capture;
  a capture mission left the runtime config 16 keys deep into capture
  variants twice because the revert was deferred to a final restore step.
- Shipped-defaults restores must be checked PER KEY against the designed
  table, not per file: after restoring template+docs, a code fallback const
  can still disagree (config_loader.rs `tool_threshold_token` fell back to
  4096 while docs/template say 512). Hardcoded values inside `#[test]`
  blocks are harmless - confirm the block is a test before treating a value
  as a production override.
- Engine-asset knowledge (shipped config defaults trio + directive
  materialization chain) lives in `references/engine-assets.md`.
- Fixture prompts must name REAL on-disk workspaces. A fixture pointing at
  a workspace that does not exist (e.g. `/tmp/rust_project/src/parser.rs`)
  makes the agent burn its turn budget searching (`find ... -name
codegen.rs`), and the cell ends `completed=false` with `creates=0` -
  an artifact that looks like compression absence but is a missing
  fixture. Build the workspace the fixture names (a real compiling
  project), record its hash in the manifest, and give every model×task
  cell an identical workspace copy.
- A manifest reporting zero CCR creates/retrieves while the transcript
  shows a successful `aphrodite_retrieve` is a MEASUREMENT BUG, not a
  behavior absence: the proxy reports stats under its own key names (its
  cache line format), which may not match the names the manifest
  extractor assumed. Audit the extraction keys against actual proxy
  output before trusting zeros in a manifest.
- Probe candidate models on the live account before committing a fanout
  set: some names 400/retired, and a model without native tool_calls
  (e.g. llama-4-scout-17b) cannot be compared on tool-use benchmarks.
  Verify native tool_calls support for every fanout member with a cheap
  one-shot probe.
- Apply the CLI model default BEFORE the API call, not after: relaying an
  unset `--model` yields `"model": ""` in the request and a provider 500
  (the request dump shows the empty field). Default at parse time
  (`args.model or MODEL`).
- **Verify the API key is actually exported, never assume.** The proxy dies
  with `no API key configured - set APHRODITE_API_KEY env var` when the key
  is absent OR commented out in the private environment file - check
  `env | grep APHRODITE_API_KEY` before the fanout; a commented-out line
  looks configured and fails at the first request.
- **Free-tier provider fanouts hit intermittent HTTP 401/429.** Re-dispatch
  failed cells in smaller clusters; treat 401/429 as transient until the
  same cell fails repeatedly, then investigate that cell.
- Fanout-run design (N models × M tasks, per-cell isolation, manifest
  fields) lives in `references/fanout-runs.md`.

## References

- `references/evidence-sources.md` - the evidence layering table + marker
  regex patterns + db paths for auditing real-session compression.
- `references/counterfactual-analysis.md` - retroactive "how much did CCR
  save" methodology: billing-vs-context rule, per-call payload
  reconstruction, retrieval tax, sensitivity band, A/B protocol for
  Aphrodite-on vs stock Hermes runs. Reusable scripts in
  ~/.hermes/tmp/session-bench/ (lib.py, analyze.py,
  compare.py).
- `references/dylib-micro-benchmark.md` - the direct-FFI timing harness
  (env-override + plugin's own load path), warmup/iteration protocol, and
  the release-vs-debug numbers table.
- `references/fanout-runs.md` - per-session model fan-out design: N
  models × M tasks matrix of AIAgent cells, provider resolution from
  Hermes config, per-cell isolation (cwd/profile/toolsets), fixture
  workspaces, model-set probing, manifest fields.
