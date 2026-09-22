# Aphrodite Benchmark Methodology (formal spec)

**Status:** living spec - the repeatable method for measuring what Aphrodite's
CCR compression actually buys under any model, any configuration, any task.
**Location:** `bench/conversational/` - `live_runner.py` (real agent turns),
`harness.py` (deterministic simulation, labeled upper bound),
`conversations.py` (task fixtures), `visualize.py` (charts).

## 0. Purpose: from compression demo to model-memory evaluation framework

The benchmark turns Aphrodite from "a system that can replace content with
markers" into a measurable **model-memory evaluation framework**: a way to
determine which models, compression policies, and retrieval interfaces make
agents *more capable* - not merely cheaper. It is a shipping, tuning, and
selling instrument. The offline harness remains useful as a deterministic
**mechanism regression test** (markerization + threshold logic behave
predictably); the live runner is the behavioral and product-value test.

## 1. Core claim (formalized)

> For a given model, task class, and configuration, CCR reduces the cost and
> context burden of an agent session **without materially reducing task
> success, correctness, or user satisfaction**.

Headline: **"net cost reduction at task-success parity"** - never a bare
"compression percentage."

### Product claims to validate (claim → measurement → decision)

| Formal claim | Measurement | Decision it supports |
|---|---|---|
| Context efficiency | prompt-token reduction per successful task | whether compression has economic value |
| Selective recall | needed retrievals / opportunities; unnecessary / all retrievals | whether the model uses memory intelligently |
| Task-quality preservation | success rate vs uncompressed baseline | whether the setting is safe to ship |
| Reasoning continuity | errors caused by lost/stale/wrongly retrieved context | whether compression damages coherence |
| Tool-use efficiency | useful tool actions / total actions | whether compression reduces or creates churn |
| Latency benefit | p50/p95 wall-clock per successful task | whether users experience an improvement |
| Cost predictability | variance of spend/tokens across comparable tasks | whether the system is operationally controllable |
| Memory robustness | performance across long conversations, many artifacts, competing markers | whether it survives realistic workloads |
| Configuration portability | results across providers and models | whether defaults generalize or need model-specific profiles |
| User trust | human preference, correction rate, repeated-use intent | whether the feature feels reliable, not clever |

## 2. Memory policies (the configuration research space)

Every configuration is a **named, versioned memory policy**. A benchmark run
must identify the exact policy, not "compression enabled."

### 2.1 Compression policy (vary)

- minimum artifact size before CCR storage
- minimum estimated token size before compression
- eligibility by content type: terminal, source files, search results, diffs,
  API payloads, logs, prior messages
- protected recent-turn window (never compress newest N turns)
- protected semantic artifacts: plans, user constraints, test failures,
  selected files, final requirements
- marker granularity: full artifact, chunks, sections, files, command blocks,
  semantic units
- marker metadata: title, source tool, timestamp, file path, content type,
  size, preview, relevance hints
- replacement strategy: immediate, delayed, or only under context pressure
- retention/eviction policy when many artifacts compete for limited memory

### 2.2 Retrieval policy (vary)

- model-directed retrieval only
- auto-expansion on marker reference
- auto-expansion by tool type or predicted relevance
- preview-first, then chunk or full-content expansion
- retrieval budgets: max calls / bytes / tokens / latency
- retrieval ranking: newest-first, source-aware, semantic, task-plan, dependency-graph
- deduplication (no repeated retrieval of unchanged content)
- failure recovery: on failed build/answer, offer candidate markers relevant
  to the failure

The meaningful test is not "did retrieval happen?" - it is **"did the model
retrieve the minimum sufficient evidence before making a dependent decision?"**

### 2.3 Agent policy (vary)

- system-prompt instructions about CCR markers and retrieval
- tool descriptions and naming
- structured task-plan maintenance
- temperature / reasoning effort
- max tool calls / iteration limit
- availability of search, file, terminal, patch, browser, retrieval tools
- presence of summaries, memory files, context-injection
- tool-result formatting: raw, truncated, structured, chunked, summarized,
  markerized

A configuration can fail not because the compression engine is wrong, but
because the model was never taught what a marker means or when to retrieve one.

### 2.4 Marker provenance (the lineage/freshness policy)

A compressed artifact is not "memory" - it is **evidence captured at a
particular moment, through a particular tool, under particular assumptions.**
These are materially different even if all refer to `src/parser.rs`:

- the complete file read at task start
- lines 80-140 read after an edit
- a search result that mentioned the file
- a compiler error attributed to the file
- a diff generated from the file
- a test failure observed before a dependency update

#### Marker shape (compact decision header, full metadata store-side)

```text
CCR:8f3a21
[file-read | src/parser.rs | lines 1-180 | turn 12 | rev a4c91d | 14.2 KB]
Parser AST and ParserError definitions; read before trait extraction
```

| Field | Include inline? | Purpose |
|---|---:|---|
| Stable marker ID | Yes | exact retrieval |
| Artifact type | Yes | file read, command output, diff, search, message |
| Source / location | Yes | path, command, URL, query, tool source |
| Capture turn | Yes | conversational chronology |
| Time captured | Usually | long sessions, external-data freshness |
| Read mode | Yes, compact | whole file, range, search hit, diff, listing, command output |
| Line/byte range | Yes when applicable | retrieve the right section |
| Workspace revision | Yes for files | staleness after edits/branch changes |
| Content digest | Store-side | exactness + deduplication |
| Short semantic synopsis | Yes | retrieval decisions without opening content |
| Tool arguments | Store-side or compact | how the artifact was obtained |
| Parent artifact / causal link | Store-side | output → command/action |
| Sensitivity / retention tag | Store-side | privacy + lifecycle |

#### "Where, when, how" as first-class fields

- **Where:** workspace-relative path + repo/branch + line range + digest (a
  path alone is not enough - `src/parser.rs` differs by repo/branch/worktree/
  commit).
- **When:** both conversation chronology AND real-world freshness -
  `captured_turn: 12`, `captured_at: <iso>`, `workspace_revision: a4c91d`,
  `precedes: edit(src/parser.rs, turn=15)`. The key concept for coding agents
  is **state time**: which workspace version existed when the file was read.
  A marker becomes visibly stale when the source changed:
  `CCR:8f3a21 [STALE] file-read src/parser.rs:1-180 captured at turn 12,
  before edits at turns 15 and 19`.
- **How:** acquisition method and limits - `tool: read_file`,
  `arguments: {path, start_line, end_line}`, `capture_mode: partial_read`,
  `completeness: partial`. Prevents the category error of treating a partial
  excerpt as the whole file. Modes: `full_file`, `line_range`, `tail`, `head`,
  `grep_match`, `directory_listing`, `terminal_stdout`, `terminal_stderr`,
  `build_diagnostic`, `test_report`, `git_diff`, `search_snippet`,
  `http_response`, `agent_summary`, `user_statement`.

#### Freshness and invalidation

When a tool edits `src/parser.rs`, the memory layer can: find markers derived
from that path → compare captured digest/revision with the new workspace state
→ mark old markers **STALE** (not silently current) → preserve as historical
evidence → suggest a current reread. This is a lightweight form of **temporal
memory**: the agent knows not only what it saw, but what state of the world it
saw.

#### Dependency graph (highest-value extension)

Store marker relationships, not independent blobs:

```text
Task request
  -> read src/parser.rs @ revision A
  -> run cargo test
  -> compiler error E0425
  -> edit src/parser.rs @ revision B
  -> rerun cargo test
  -> passing result
```

Links: `produced_by`, `derived_from`, `supersedes`, `invalidates`,
`verified_by`, `relevant_to`, `contradicts`. The agent then retrieves not "the
parser file" but *the last successful test after the parser change*, *the exact
error that motivated an edit*, *the pre-edit snapshot for regression analysis*,
*the current view excluding known stale reads*, *all artifacts attached to a
subtask*.

#### Semantic labels (controlled vocabulary)

```text
kind: source_file
status: current | stale | historical | superseded | verified
relevance: task_requirement | implementation | diagnostic | validation | reference
authority: user | repository | tool | external | model_generated
confidence: exact | partial | inferred | summarized
```

These answer cheaply: raw evidence vs summary? authoritative vs hypothesis?
current vs historical? complete vs partial? requirement vs implementation vs
error vs validation? A model should prefer **current, authoritative, exact**
evidence before editing code or asserting a fact.

#### Metadata budget (three tiers)

| Tier | Contents | Where it lives |
|---|---|---|
| Inline header | ID, type, source, state, turn/revision, terse synopsis | model context |
| Catalog entry | full provenance, lineage, tool args, size, digest, related markers | searchable compressed index |
| Retrieved payload | original raw content + complete metadata | returned on demand |

Target ~20-60 tokens inline, scaling with artifact importance/ambiguity.

#### Example markers

```text
CCR:41f2ae
[source-file | src/parser.rs:1-180 | full read | rev 93bd1c | turn 12 | CURRENT]
Defines AST tokens and ParserError; captured before the planned trait extraction

CCR:7ac011
[build-diagnostic | cargo test | cwd project/ | exit 101 | turn 18 | CURRENT]
E0277 in src/codegen.rs after ParserError trait refactor; 24 diagnostics

CCR:4ee091
[source-file | src/parser.rs:1-180 | rev 93bd1c -> now e7a602 | STALE]
Historical pre-refactor snapshot; retrieve only for comparison or regression analysis

CCR:d3c98b
[validation | cargo test | 42 passed, 0 failed | rev e7a602 | turn 29 | VERIFIED]
Post-refactor test result; authoritative completion evidence
```

#### Retrieval-behavior improvements enabled by provenance

prefer current evidence; avoid stale reads (reread, don't expand obsolete);
retrieve narrowly (lines 80-140, stderr only); navigate causally (error →
producing command → subsequent fix); verify claims (latest validation marker
before asserting "tests pass"); reduce duplicate work (surface already-read
current file); detect contradictions (rank later, higher-authority, verified).

### 2.5 Workflow-aware markers (the cognitive scaffold)

Provenance + workflow meaning gives the model a **compact explanation of the
agent's past workflow** - so even a weaker model need not reconstruct why an
artifact exists, what it was used for, or what should happen next. This
directly addresses weaker models' poor long-horizon state tracking and weak
inference from raw tool history.

#### The key shift

A normal marker says "src/parser.rs, lines 1-180." A workflow marker says:

```text
CCR:abc123
Read during "diagnose ParserError overlap" to compare parser.rs with codegen.rs.
Role: evidence for planned trait extraction.
State then: pre-edit workspace, revision 93bd1c.
Next expected use: retrieve if choosing error-type ownership or validating the refactor.
```

#### `read_as` / workflow_role metadata (the most important field)

How the information was interpreted at the moment it entered context:

```yaml
marker_id: CCR:abc123
artifact: {kind: source_file, path: src/parser.rs, range: 1-180}
capture: {tool: read_file, turn: 12, workspace_revision: 93bd1c}
read_as:
  task_phase: diagnosis
  purpose: compare error-type definitions across parser and codegen modules
  expected_decision: choose common error representation
  confidence: direct_evidence
  completeness: partial_file
workflow:
  preceded_by: compiler_error:E0277
  supports: planned_refactor:error_trait_extraction
  next_likely_actions: [retrieve src/codegen.rs comparison,
                        inspect callers of ParserError,
                        implement shared trait]
freshness: {status: stale_after_turn_15}
```

#### Why weaker models benefit

| Weak-model problem | Workflow marker support |
|---|---|
| Loses the goal after many turns | states task phase + original purpose |
| Cannot infer why a file matters | records the decision/question the read supported |
| Treats old content as current | includes workspace revision + stale/current status |
| Rereads everything | suggests the next likely retrieval/action |
| Confuses evidence with conclusions | labels source evidence vs hypothesis vs edit vs diagnostic vs verification |
| Repeats failed approaches | stores failed attempt and why |
| Cannot connect tools into a plan | connects predecessor, current artifact, downstream actions |
| Makes unsupported claims | points to exact evidence + validation markers |

This is a **cognitive scaffold**: the system carries workflow structure; the
model supplies reasoning and tool choice.

#### Workflow maps (task-state graph, not just transcript)

```text
User request: Unify ParserError and CodegenError
Diagnosis:  Read parser.rs ─┐
                            ├─> Compare overlapping error variants
            Read codegen.rs ┘
Plan:       Introduce shared error trait
Execution:  Edit parser.rs; Edit codegen.rs
Validation: cargo test failed ─> Read diagnostics ─> Fix bound ─> cargo test passed
```

The model receives a small current map, not the whole graph:

```text
WORKFLOW STATUS - error-type refactor
Completed: compared error definitions; chose shared trait approach
Current:   fixing generic trait-bound error after first implementation
Evidence:  CCR:parser-read (pre-edit), CCR:codegen-read (pre-edit), CCR:build-e0277 (current failure)
Recommended next: retrieve CCR:build-e0277, inspect the trait implementation near the reported line
```

#### Epistemic labels (never preserve model opinions as facts)

Distinguish **what happened** from **what the model believed**:

- `observed` - direct raw tool output or user statement
- `inferred` - model interpretation, potentially wrong
- `hypothesis` - an idea to test
- `planned` - intended next step
- `attempted` - action taken
- `failed` - disproven path or failed execution
- `verified` - confirmed by test/build/user approval/authoritative source

Without this, an early incorrect conclusion can be reintroduced as
authoritative memory. A low-capability model should prioritize direct, current,
verified evidence over old hypotheses.

#### The workflow receipt

```text
CCR:7ac011
WHAT:  Build failure E0277, missing trait bound
WHERE: cargo test, repo root, src/codegen.rs:84
WHEN:  Turn 18, revision e7a602, after shared-error-trait edit
HOW IT WAS READ: diagnostic evidence explaining why the first refactor failed
WORKFLOW ROLE: blocks validation of the error-type unification task
STATE: Current; no later successful build exists
NEXT:  retrieve before changing generic bounds; supersede only after a new test run
```

#### Reusable workflow templates

bug fix (reproduce→inspect→hypothesize→patch→test→verify); repository
exploration (map→inspect→answer with evidence); release process
(check→gates→build→validate→tag→restore); research (gather→assess→extract→
resolve→synthesize); incident response (signals→hypothesis→mitigate→validate→
document). The model receives a task-specific map instead of inventing
disciplined workflow management from scratch.

#### Adaptive assistance levels

| Model capability | Marker design |
|---|---|
| Strong reasoning | compact provenance, optional graph lookup, model chooses actions |
| Mid-tier | provenance + task phase + recommended markers + stale warnings |
| Small/weak | explicit workflow state, allowed next actions, evidence hierarchy, suggested retrieval before action |
| Highly constrained | structured state machine with limited choices + automatic validation gates |

This moves routine state tracking from expensive model reasoning into
dependable system structure, while allowing strong models to use the map
flexibly.

## 3. Metric families

### 3.1 Cost

| Metric | Definition | Source |
|---|---|---|
| gross tokens | baseline vs compressed prompt/completion | provider `usage` |
| net savings | compressed − retrieved (per task) | proxy stats + usage |
| cost delta | USD per successful task | tokens × price |
| p50/p90 net savings | distribution across tasks | manifest |

### 3.2 Memory-retrieval metrics (precise definitions)

| Metric | Formal definition | Interpretation |
|---|---|---|
| Retrieval precision | relevant retrievals / all retrievals | little wasted expansion |
| Retrieval recall | required artifacts retrieved / required available | no missed evidence |
| Retrieval F1 | harmonic mean of precision and recall | single retrieval-quality score |
| Premature retrieval rate | retrievals before evidence needed / all | overly eager expansion |
| Late retrieval rate | retrievals only after failed dependent action / needed | delayed memory use |
| Missed-context failure rate | failures attributable to unretrieved available artifact / tasks | key quality-risk measure |
| Redundant retrieval rate | duplicate retrievals of unchanged content / all | poor marker navigation |
| Retrieval expansion ratio | retrieved tokens / compressed tokens | whether retrieval negates savings |
| Memory navigation time | time from information need to correct retrieval | practical fluency |
| Marker selection accuracy | correct marker selected / retrieval attempts | tests naming, previews, catalogs |

Each task needs an **evidence map**: artifacts that are necessary, helpful,
irrelevant, or misleading per milestone - separating successful reasoning from
lucky guesses.

### 3.3 Tool-use definitions (decomposed)

- **Useful action** - produces information/state change contributing to a
  verified successful milestone (read a file later modified correctly, run a
  test exposing a real failure, retrieve context needed for a correct decision).
- **Redundant action** - repeats an unchanged query/read/retrieval/command
  without adding information relevant to the next decision.
- **Recovery action** - follows a detected error/failed verification and moves
  toward a correct state (recovery is not bad; excessive loops are).
- **Invalid action** - wrong syntax, arguments, location, permissions, or
  assumed preconditions.
- **Counterfactual action** - an extra action *caused by compression* (e.g.
  rereading a file because earlier output was hidden behind a marker the model
  failed to retrieve).

Metrics: calls per successful task, calls per completed subgoal,
useful-action ratio, repeat-read ratio, failed-command ratio, edit-verify
ratio, search-to-resolution ratio, recovery-loop count, counterfactual
tool-call count, time-to-first-productive-action, time-to-verified-completion.

Credible statement this enables: **"Policy B lowered prompt tokens by 35%
while reducing redundant tool calls by 18%, with no task-success regression."**

### 3.4 Quality & satisfaction (three levels)

**Automated proxies:** completion, verification pass rate, user interruptions,
clarification requests, turns to completion, rework after final answer,
contradiction/unsupported-claim rate, user-request adherence.

**Expert review** (blinded to baseline vs compressed): correctness,
completeness, evidence grounding, appropriate confidence, clarity, efficiency,
apparent context loss, tool-choice sensibility, trust on a similar task.

**Real-user validation** (privacy-safe aggregates): thumbs-up/down, correction
rate, repeated questions about provided facts, abandonment, "you forgot /
that's wrong / read the earlier message" frequency, willingness to reuse.

This separates **compression that works technically** from **memory behavior
people trust**.

### 3.5 Latency

- end-to-end wall-clock per task and per turn (p50/p95)
- retrieval byte-exactness (retrieved == stored, verified by hash)

## 4. Experiment matrix

A run = **model × memory policy × scenario × task**, pinned to one run ID.

### Model dimensions

provider, model version, context-window size, temperature, tool-calling mode;
strong vs weaker reasoning models (inference of when to expand markers is a
major determinant of compression value); native tool-call formats and
provider-specific usage accounting.

### Task dimensions (known success criteria, executable workspaces)

code repair; repository exploration; multi-file refactoring; release
operations; research/synthesis; long support workflows; compression-aware
tasks. Existing fixtures (coding, debugging, exploration, release_flow,
compression_aware) are the foundation - they need executable workspaces +
independently checkable success tests.

### Long-context stress tests (adversarial workloads)

- **Needle recall:** one essential detail among many large irrelevant
  artifacts; retrieve at the right moment
- **Conflicting revisions:** outdated vs updated specs; find the authoritative
  newer source
- **Delayed dependency:** reveal a requirement early, bury it under tool
  output, test compliance much later
- **Multi-artifact synthesis:** combine facts from several independently
  compressed files
- **Near-duplicate artifacts:** similar logs/files with one important
  difference; test retrieval discrimination
- **Misleading preview:** previews that look relevant but lack the critical
  detail; test escalation to full retrieval
- **Failure-triggered recall:** plausible wrong attempt, then retrieve the
  right evidence to recover
- **Budget pressure:** grow context until full-history baseline is expensive/
  impossible; compare quality-degradation curves
- **Long-running workspace:** files change after compression; test staleness
  detection and invalidation
- **Conversation handoff:** a new agent/resumed session uses markers and
  catalogs to recover necessary state

### Required scenarios (every model × task)

1. **Baseline** - no Aphrodite compression
2. **Cache-only** - CCR storage/marker behavior, no token-proxy
3. **Token-proxy-only** - context-pressure behavior, no cache compression
4. **Full** - production-equivalent combined behavior

Then an **ablation sweep**: change exactly one setting from full at a time.

### Marker-format ablation (the provenance experiment)

Test the marker design axis directly:

1. Raw full context (no compression)
2. Hash-only compressed markers
3. Provenance-aware markers (type, source, capture turn, revision, synopsis)
4. Provenance + workflow-map markers (read_as, task phase, next actions, epistemic labels)

Evaluate separately for **strong and weak models**:

- task completion and verification rate
- wrong-file / wrong-marker retrieval rate
- redundant read and search rate
- failed-command and recovery-loop rate
- stale-context errors
- time and tokens to completion
- plan adherence
- ability to resume a paused task
- difference in performance between high- and low-capability models

The especially valuable finding is **capability equalization**: if
workflow-aware markers let a cheaper model approach the task success of a more
expensive one on long agent workflows, Aphrodite produces value beyond token
compression.

## 5. Net-value scorecard

```
Net value = cost savings + latency benefit + tool-efficiency benefit
          − retrieval cost − quality-degradation risk
```

Publish the components first; weighted score only for internal ranking with
explicit weights.

### Release gates

- task-success difference from baseline within agreed tolerance
- no statistically meaningful decline in human preference
- positive median **and** p90 net token/cost savings
- no increase in failed-tool or redundant-tool-call rates
- retrieval precision and recall above task-dependent floor
- no regression for long-context or high-stakes families
- no severe regression on the long-context stress suite

## 6. Derived business value

- **Model selection:** choose models on *cost-adjusted task success under
  compressed memory* - a cheaper model that retrieves poorly may cost more per
  successful task.
- **Default profiles:** publish model-specific policies - Conservative
  (high retention), Balanced (moderate compression, guarded retrieval),
  Aggressive (heavy compression for long technical workflows), Research
  (preserve citations/evidence chains), Coding (preserve diffs/errors/test
  results/paths/command history), Support (preserve user constraints/
  commitments/chronology).
- **Automatic tuning:** raise thresholds for weak-recall models; chunk-level
  markers for over-retrievers; richer previews where marker selection accuracy
  is low; protect more recent evidence when late-retrieval failures rise;
  trigger prefetch only where it pays; disable a config where it causes churn
  or quality loss.

Decision changes from "the proxy compressed output" to **"this memory policy
is eligible for production defaults for model X on workload Y."**

## 7. Reporting artifacts (four complementary views)

1. **Executive scorecard** - task success, net cost, latency, tool efficiency,
   satisfaction
2. **Retrieval confusion matrix** - needed/retrieved, needed/not retrieved,
   unnecessary/retrieved, unnecessary/not retrieved
3. **Trajectory explorer** - per-turn prompt size, marker creation, retrievals,
   tools, failures, task milestones
4. **Pareto frontier** - configurations where no alternative provides lower
   cost without sacrificing quality, latency, or tool efficiency (there may be
   no globally best policy)

## 8. Protocol & reproducibility

1. fixed seed/tasks; 2. isolation (cwd=`./bench/conversational`,
   skip_context_files, skip_memory); 3. provider resolution (default = Hermes'
   config; overridable per run); 4. same task prompt across scenarios/configs;
   5. baseline control in every matrix; 6. honest accounting (token source
   labeled; simulation = upper bound; live = real).

Per-run artifacts: immutable config snapshot, model/provider metadata, task
version + workspace hash, full tool and CCR event trace, per-turn usage +
latency, final artifacts + automated verification, human-eval records,
machine-readable manifest + dashboard.

## 9. Stronger value statements

- **Context efficiency:** "For model X on task family Y, this configuration
  reduces median prompt tokens by Z%."
- **Economic value:** "After retrieval overhead, the configuration reduces
  median provider cost per successful task by Z%."
- **Quality preservation:** "Task-success parity remains within tolerance
  relative to the uncompressed baseline."
- **Selective-memory competence:** "The model retrieves needed compressed
  evidence with defined precision, recall, and timing."
- **Operational efficiency:** "Compression reduces or does not increase
  tool-call volume, retries, and recovery loops."
- **Latency value:** "The configuration reduces p50/p95 end-to-end completion
  time, or identifies when added proxy/retrieval work outweighs token savings."
- **Robustness:** "The configuration remains beneficial across models, context
  windows, providers, and task classes."
- **Tuning guidance:** "The benchmark identifies a Pareto frontier:
  configurations where no other provides more savings without degrading
  quality or latency."
- **Explainability:** "Each result traces to exact compressed artifacts,
  markers, retrievals, tools, and task outcomes."
- **Safe adoption:** "A model/configuration is eligible for default enablement
  only when it clears explicit quality, retrieval, and efficiency gates."

## 10. Formal success statement

> For model *M*, task family *T*, and memory policy *P*, Aphrodite is
> beneficial if it achieves non-inferior task success and user-rated quality
> relative to the uncompressed baseline, while producing statistically
> meaningful improvements in net cost, context utilization, latency, or
> tool-use efficiency.

Stronger:

> Aphrodite provides selective externalized memory when an agent can compress
> non-immediately-needed artifacts, retrieve the right evidence at the right
> moment, and preserve completion quality with lower total resource use than
> retaining all raw history in context.

And the workflow-marker claim:

> **Workflow-aware markers enable smaller models to complete long, tool-heavy
> tasks with higher success and lower redundant tool use than source-only or
> hash-only markers** - capability equalization, not just token savings.

The larger product statement:

> Aphrodite externalizes agent working memory into provenance-aware workflow
> objects: each compressed artifact remembers not only its content, but why it
> entered the workflow, what decision it supports, what state it belongs to,
> and what actions should follow. This is **persistent procedural context** -
> helping less capable models operate reliably inside sophisticated workflows
> while helping stronger models use less context and make fewer unnecessary
> tool calls.

That is the real product: not compression as a storage trick, but **adaptive,
measurable memory management for tool-using AI agents**.

## 11. Current tooling & gaps

| Tool | Kind | Measures |
|---|---|---|
| `live_runner.py` | live agent turns (AIAgent) | real tokens, completion, elapsed, CCR create/retrieve, per-cell manifest |
| `harness.py` | deterministic simulation | upper-bound token savings (no retrieval - labeled) |
| `run_all.sh` | orchestration | dry-run validation + full simulation runs |
| `visualize.py` | charts | token comparison, timeline, compression efficiency, radar, dashboard |

**Gaps to close (in order):**
1. **Live runner auth fix** - the default `auth-cloudflare-workers-ai` is
   OAuth-based; the runner must resolve credentials exactly as Hermes does,
   not a guessed env var. (Blocking live runs.)
2. **Executable task workspaces** - fixtures become real directories with
   verifiable success checks.
3. **Task evidence maps** - per-milestone necessary/helpful/irrelevant/
   misleading artifacts for objective retrieval metrics.
4. **Named memory-policy axis** - `--policy <name>` with versioned toml
   variants so the ablation sweep is executable.
5. **Marker-format axis** - `--markers <hash|provenance|workflow>` so the
   4-way provenance ablation is runnable (hash-only → provenance →
   provenance+workflow-map).
6. **Dashboard v1** - task-success parity, net savings, retrieval precision/
   recall, tool-call efficiency, p50/p95 latency, Pareto frontier.
7. **Per-fixture graders** - objective correctness beyond binary completion.

## 12. Known limitations (state in every report)

- Simulation harness models no retrieval → its savings are upper bounds.
- Live runs cost real tokens/money per matrix cell - keep the matrix small.
- Task-success grading is binary (`completed`) until graders land.
- Live runs require a working provider credential resolved Hermes-style.