# Per-Session Model Fan-Out Benchmark Runs

## When to Use

Use when a benchmark must compare how different models perform with tool
calls, CCR compression, and CCR retrievals in real agent sessions — the
N-models × M-tasks fan-out design operated from inside a Hermes session.

Design for benchmarking how DIFFERENT models perform with tool calls, CCR
compression, and CCR retrievals, using real agent sessions.

## Shape

- **Benchmark-specific session run, not all-bench**: one bench invocation
  fans out to N models (typically 5), each model × task = one cell.
- **Cell = fresh AIAgent session** pinned to that model, launched
  programmatically from inside a Hermes session ("the harness is operated
  within a `hermes` session; it launches a normal session as a user would").
- **Granularity + fanout combined**: per-cell evidence (not whole-bench
  aggregates) so results are comparable cell-by-cell: same task, same
  workspace, same prompt across models.
- **No proxy/cache token modes for baseline runs**: compression runs through
  the plugin's inline CCR in each session; proxy modes are a later,
  separate scenario.

## Provider resolution

- Resolve from Hermes' own config: `~/.hermes/config.yaml` (model.base_url /
  model.default) + `~/.hermes/.env` (API key). Never hardcode a vendor.
- The live provider module is the one Hermes uses for auth (e.g.
  auth-cloudflare-workers-ai); list models via its models_url()
  (format=openrouter&per_page=1000).
- User rule: "use the default auth provider".

## Model-set selection

1. Probe each candidate with a cheap one-shot on the live account.
2. Require native tool_calls: a model without them (e.g. llama-4-scout-17b)
   cannot be compared on tool-use benchmarks — exclude.
3. Drop 400/retired names; keep the cheapest live model that Hermes is
   already configured on as the reference cell.

## Per-cell isolation

- `cwd` = results/<run-id>/<model-slug>/<task>/ — terminal tool follows it.
- File tools are NOT cwd-restricted (absolute paths) — cwd alone is not
  containment. Full isolation = dedicated profile (isolated HERMES_HOME) +
  restricted enabled_toolsets + per-cell cwd.
- Skills remain available: they load from the skills dir, independent of
  cwd; skip_context_files only skips AGENTS.md/context files.

## Fixture workspaces

- Fixture prompts must name REAL on-disk workspaces
  (bench/conversational/workspaces/<task>/), each a real compiling project
  with the files the prompt mentions. A missing workspace makes the agent
  burn turns searching and the cell ends completed=false with creates=0
  (fixture gap, not compression absence).
- Record the workspace hash in the manifest; give every cell an identical
  copy so comparisons are fair.

## Manifest fields (per cell)

model, task, completed, elapsed, message_count, tool_calls (extracted from
messages), CCR creates, CCR retrieves, token usage, workspace hash, run-id.

## Pitfalls

- Stat key names: the proxy reports cache/CCR stats under ITS own key names;
  if the manifest extractor assumes different names it reports zeros while
  retrieves demonstrably happened. Audit extraction keys against real proxy
  output before trusting zeros.
- Empty model: default at parse time (`args.model or MODEL`); relaying an
  unset model yields "model": "" and a provider 500.
- Verify the session dylib before the run (aphrodite_rebuild → version)
  and confirm which tool count the profile exposes (dev 14 vs release 13).
