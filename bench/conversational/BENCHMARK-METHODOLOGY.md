# Aphrodite Benchmark Methodology (formal)

**Status:** living spec - the repeatable method for measuring what Aphrodite's
CCR compression actually buys, under any model, any configuration.
**Location:** `bench/conversational/` - `live_runner.py` (real agent turns),
`harness.py` (deterministic simulation), `conversations.py` (task fixtures),
`visualize.py` (charts).

## 1. Core question (formal)

> Under a given **provider + model + Aphrodite configuration + task set**,
> does CCR compression reduce tokens/latency/cost **without degrading task
> success** - and does the model's *retrieval behavior* (what it chooses to
> re-expand from `<<<CCR:hash>>>` markers) account for most of the difference?

This decomposes into the claims the benchmark must evidence:

- **C1 (compression claim):** tool outputs and context history are compressed
  into markers below the configured thresholds; the LLM receives a smaller
  prompt than it would without Aphrodite.
- **C2 (retrieval claim):** the LLM re-expands markers selectively - it
  retrieves what the task needs and skips what it does not - so net savings
  (compressed − retrieved) stay positive.
- **C3 (parity claim):** task success under compression equals task success
  baseline. Compression that saves tokens but fails tasks is a regression.
- **C4 (config claim):** the configuration (thresholds, engine pct,
  auto_expand, directives, catalog mode, flow budget) moves C1/C2/C3 in
  predictable directions - and the benchmark quantifies the trade-offs so
  defaults can be tuned with evidence.
- **C5 (model claim):** different models exhibit different retrieval
  discipline; the benchmark makes model-vs-model behavior comparable on the
  same tasks and configuration.

## 2. Metric families

| Family | Metric | Measures | Source |
|---|---|---|---|
| **Cost** | gross tokens (baseline vs compressed) | C1 | API `usage` / tiktoken |
| | net savings = compressed − retrieved | C2 | proxy stats + usage |
| | cost delta (USD per task, per model pricing) | C1+C4 | tokens × price |
| **Retrieval** | retrieve count / retrieve volume | C2 | proxy `/stats` |
| | retrieve precision (retrieved markers actually used) | C2 | trajectory |
| | marker skip rate (markers never retrieved) | C2 | trajectory |
| **Quality** | task success (binary: fixture goal reached) | C3 | agent `completed` |
| | satisfaction proxy (turns to completion, re-reads, dead-ends) | C3 | trajectory |
| | answer correctness (fixture-specific grader) | C3 | grader |
| **Behavior** | tool-call count / tool-call redundancy | tool-use quality | trajectory |
| | tool schema adherence (valid args, correct tool chosen) | tool-use definitions | trajectory |
| | re-read rate (file re-read after marker compression) | retrieval quality | trajectory |
| **Latency** | wall-clock per task, per turn | real use | timing |
| **Memory** | recall quality of retrieved content (byte-exact vs paraphrased) | memory retrieval | retrieved-vs-stored hash |

## 3. Experiment matrix

A run = **model × configuration × scenario × task**, all fixed to one run ID:

```
models          any provider/model (--provider/--model/--base-url/--api-key)
configurations  --config <aphrodite.toml variant> (default: shipped defaults)
scenarios       baseline | full | hermes_proxy | proxy_api
tasks           coding_task | exploration_task | debugging_task |
                release_flow_task | compression_aware_task
```

**Configuration axis (the tuning knobs the matrix varies):**

| Knob | TOML key | Effect the benchmark should quantify |
|---|---|---|
| tool-output threshold | `tool_threshold_cache` / `tool_threshold_token` | how much tool output becomes markers (C1) vs how often the model must retrieve (C2) |
| engine threshold | `engine_threshold_pct` | when context offload kicks in (C1) vs retrieval burden (C2) |
| auto_expand | `auto_expand` | marker-free UX vs lost compression (C1 vs C2 trade-off) |
| protect first/last | `engine_protect_first`/`_last` | offload aggressiveness vs recent-context fidelity (C3) |
| directives | `[directives] active` | retrieval vocabulary taught to the model (C2 - does guidance improve skip rate?) |
| catalog mode | `catalog_mode` | catalog size in context vs recall ability |
| flow budget | `flow_budget_chars` | injected-context overhead vs model guidance |
| code multiplier | `code_multiplier` | code-vs-text threshold split |

**The headline output for each cell of the matrix:** a row in the comparison
table - `net_savings%`, `task_success`, `retrieve_count`, `tool_calls`,
`satisfaction_proxy`. Charts (`visualize.py`) show token comparison, timeline,
compression efficiency, radar.

## 4. Protocol

1. **Fixed seed/tasks:** same fixtures every run (`conversations.py`) so runs
   are comparable across models and configs.
2. **Isolation:** agents run cwd=`./bench/conversational`, `skip_context_files`
   and `skip_memory` (no SOUL.md/AGENTS.md/memory bleed); results write only
   under `results/<run_id>/`.
3. **Provider resolution:** default = Hermes' own config (what a real session
   uses); overridable per run (`--provider/--model/--base-url/--api-key`) so
   other people can benchmark their model.
4. **Same task prompt:** the live runner extracts the first user turn of each
   fixture as the task; the model sees the identical instruction across
   scenarios and configs.
5. **Baseline control:** every matrix includes the `baseline` scenario (no
   proxy, no CCR) so compression cells are always measured against the
   uncompressed reference.
6. **Reproducibility:** one `manifest.json` per run (model, config, scenarios,
   per-cell metrics) - the artifact that makes model-vs-model and
   config-vs-config comparison possible.
7. **Honest accounting:** token numbers are labeled by source (`response_usage`
   vs `estimate`); the simulation harness (`harness.py`) is explicitly an
   upper-bound (no retrieval modeled), the live runner is the real number.

## 5. Value statements (what the benchmark decides)

- **"Does compression pay for itself?"** - net savings = compressed −
  retrieved. Positive = the model's selective retrieval keeps CCR worthwhile;
  negative = the model retrieves everything and compression is overhead.
- **"Which defaults are right?"** - varying the configuration axis shows the
  threshold/engine/auto_expand trade-off curves (savings vs success vs
  retrieval burden) so defaults are chosen on evidence, not intuition.
- **"Which model is best for this workload?"** - same tasks + config, different
  models: comparable net savings, task success, retrieval discipline, tool-call
  quality. (This is the "other people can try different models" use case.)
- **"Does guidance help?"** - directives that teach retrieval vocabulary
  (markers/catalog/retrieve) should measurably improve skip rate and net
  savings; the benchmark quantifies the lift.
- **"Does compression hurt tool use?"** - tool-call counts, re-read rates, and
  schema adherence under compression vs baseline answer whether markers degrade
  the model's ability to use tools correctly.
- **"Does compression hurt memory?"** - retrieval byte-exactness and recall
  quality answer whether the model works from accurate content or paraphrased
  guesses.
- **"Is the UX acceptable?"** - satisfaction proxies (turns to completion,
  dead-ends, re-reads) capture whether compressed sessions *feel* as good as
  uncompressed ones, not just whether they finish.

## 6. Current tooling

| Tool | Kind | Measures |
|---|---|---|
| `live_runner.py` | live agent turns (AIAgent) | real tokens, completion, elapsed, CCR create/retrieve, per-cell manifest |
| `harness.py` | deterministic simulation | upper-bound token savings (no retrieval modeled - labeled) |
| `run_all.sh` | orchestration | dry-run validation + full simulation runs |
| `visualize.py` | charts | token comparison, timeline, compression efficiency, radar, dashboard |

**Known limitations (must be stated in any report):**
- simulation harness models no retrieval → its savings are upper bounds
- live runs cost real tokens/money per matrix cell (keep matrix small)
- task-success grading is currently binary (`completed`) - per-fixture graders
  are the next extension for correctness depth