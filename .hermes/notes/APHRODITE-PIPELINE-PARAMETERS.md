# Aphrodite pipeline parameters - the tunable surface

Aphrodite is a pipeline: ingest → classify → compress → store → retrieve → render. Every stage today
carries hardcoded decisions - the marker-group tables, the thresholds, the caps, the sampling widths.
This document specifies how those decisions become a **first-class, editable, benchmarkable
parameter surface** instead of constants scattered through the source.

The loop it exists to enable:

```
edit parameters → run the benchmark → read the score → keep or revert
```

Nothing in this document is implemented yet. It is the specification the implementation card follows.

## 1. What this owns

- The classification of every business-logic decision: tunable, fixed protocol, or internal.
- The three-layer resolution model and its precedence.
- The on-disk format and location of the shipped defaults, and the generated human catalogue.
- The typed access layer that replaces string keys at call sites.
- The benchmark harness that makes a parameter change measurable.
- Generated case sets - how a parameter space becomes many cases without a Cartesian explosion.
- The migration phases.

It does **not** own: the style rules (`RUST-STYLE-FUNCTIONAL.md`), the naming rules
(`HOUSE-STYLE-REFACTOR.md`), or the release ceremony.

## 2. The classification - not everything may become a parameter

The single most important rule here, because the alternative is a configurable binary that cannot
read its own stored data.

| Class | Meaning | Examples | Configurable? |
| --- | --- | --- | --- |
| **Tunable** | A policy choice. Two values are both correct; one scores better. | thresholds, caps, marker-group word lists, sampling widths, reducer limits, retry counts | **yes** |
| **Protocol** | Bytes that cross a boundary another process or a stored payload depends on. | the marker grammar `<<<CCR ` / `>>>`, `SegMarker`, ABI symbol names, the cache-key field list, JSON keys on the wire | **no** |
| **Internal** | Implementation detail with no observable policy meaning. | FNV offset/prime, atomic counter widths, `OnceLock` statics, test fixtures | **no** |

The test for **Tunable**: *if two deployments could legitimately want different values, and no
stored or in-flight data breaks when it changes, it is tunable.*

The test for **Protocol**: *if changing it invalidates data already written, or breaks a client
already compiled against it, it is protocol - and it stays a constant with a comment saying why.*

`SegMarker` (`__APHRODITE_SEG__`) is the sharpest example: it looks like a tunable string and is not.
Chain-split output containing it is already stored; changing the value makes old segments
unrecognisable. Same for the marker prefix - retrieval of every existing entry depends on it.

## 3. The resolution model

Four layers, highest precedence last, each **optional** and each defaulting to *inherit*:

```
1. compiled default      crates/aphrodite/templates/parameters.toml, include_str!'d
2. user configuration    ~/.hermes/aphrodite/aphrodite.toml  [parameters]
3. environment           APHRODITE_PARAMETERS_<GROUP>_<KEY>   (scalars only)
4. call-time override    the ABI / MCP surface, for benchmarks
```

Rules that make this safe:

- **Absent means inherit, never zero.** A missing key takes the layer below it. `0` and "unset" are
  different values and the type says so: a limit is `Option<NonZeroUsize>`, not `usize` where `0`
  secretly means unlimited.
- **The binary ships its defaults.** Layer 1 is `include_str!`, not a filesystem read. The dylib
  loads into a host process with no working directory of its own; a missing parameters file must
  never be a startup failure.
- **A malformed override is loud and non-fatal.** A bad value in `aphrodite.toml` logs a warning
  naming the key, the offending value and the default it fell back to, and the pipeline keeps
  running. Silently ignoring it is how a benchmark measures the wrong configuration.
- **Resolution happens once, at boot**, into an immutable `Parameters` value passed down the
  pipeline. No stage reads the environment or the file itself - that is how two stages end up
  disagreeing about the threshold.

## 4. The files

### 4.1 Source of truth: TOML, not Markdown

The request was a `.md` file beside the code, included as a string. **I recommend TOML as the source
of truth and Markdown as a generated view**, because a Markdown table as the canonical format means
writing and maintaining a Markdown-table parser, and it cannot express types, ranges or nesting
without inventing a mini-language. The observable outcome is identical - one editable text file next
to the code - and the parser is `toml`, which the crate already depends on.

```
crates/aphrodite/templates/parameters.toml     canonical defaults, include_str!'d
crates/aphrodite/templates/parameters.md       GENERATED catalogue, never hand-edited
```

The Markdown is produced by `Maintain/parameters/render.py` (or an `aphrodite parameters render`
subcommand) and is what a reviewer reads and what a diff shows. A `--check` mode fails when the two
disagree, so the catalogue cannot drift.

If a Markdown-as-source is insisted on later, the only viable shape is one fenced `toml` block
inside it, extracted by the generator - the table form is not parseable without a spec.

### 4.2 Shape

```toml
[threshold.cache]
# Bytes above which a cached proxy body is compressed before storing.
compress = 8192

[threshold.token]
compress = 1024

[threshold.inline_ccr]
bytes = 256

[classify.terminal.failure]
# Ordered; first match wins. Outranks `build` because a failed test run prints both.
patterns = ["exit code:", "Error:"]

[classify.terminal.build]
patterns = ["   Compiling", "    Finished", "   Running", "test result:", "   Building", "   Installing", "warning:", "error["]

[classify.terminal.build_error]
patterns = ["error[", "error: could not"]

[classify.essential_tools]
names = [ ... ]

[cap]
payload_bytes = 16777216      # 16 MB
response_bytes = 67108864     # 64 MB
prefetch_bytes = 10485760     # 10 MB
poll_accumulator_bytes = 262144

[preview]
max_chars = 120               # 0 = unlimited, the existing contract
sample_head = 28
sample_tail = 28
sample_flat = 60

[stage2]
minimum_bytes = 80
errors_retained = 20
warnings_retained = 10
summary_retained = 10
signatures_retained = 50

[struct_extract]
signature_chars = 60
parameter_chars = 35
budget_chars = 300
scan_lines = 60

[session]
parent_depth = 16
watcher_debounce_ms = 500
recent_markers = 200
tool_events = 200
referenced_files = 100

[scheduling]
max_background_tasks = 4
stale_turn_age = 8

[retry]
connect_attempts = 3
```

Every key carries a comment stating the unit and what changes when it moves. A key without a stated
unit is a defect: `8192` means nothing until the file says *bytes*.

### 4.3 Typed access

A call site never sees a string key:

```rust
// crates/aphrodite/Source/Parameters/Threshold/Cache/Fn.rs
pub(crate) fn Fn(Parameters:&Parameters::Struct::Struct) -> NonZeroUsize {
	Parameters.Threshold.Cache.Compress
}
```

The registry is generated from the TOML at build time into `Source/Parameters/...` leaves (one item
per file, per the house style), so:

- a key that does not exist is a **compile error**, not a runtime default;
- adding a parameter means editing one TOML block and re-running the generator, not threading a
  struct through six modules;
- the generator's output is committed, so a build without the generator still works.

## 5. The catalogue

Measured at the audit HEAD. The rewrite in progress moves some paths; the card re-measures before
implementing. **T** = tunable, **P** = protocol, **I** = internal.

### 5.1 Classification tables (the shape the request named)

| Current path | Const | Value | Class |
| --- | --- | --- | --- |
| `Tool/Unwrap/Matcher/Terminal/Classify/Resolve/Fn.rs` | `FAILURE` | `["exit code:", "Error:"]` | **T** |
| same | `BUILD` | `["   Compiling", "    Finished", "   Running", "test result:", "   Building", "   Installing", "warning:", "error["]` | **T** |
| same | `BUILD_ERROR` | `["error[", "error: could not"]` | **T** |
| `Tool/Unwrap/Matcher/Single/Fn.rs` | `PRIORITY` | `["description", "summary", "result", "message", "preview", "found"]` | **T** |
| `Configuration/Path/Classify/Fn.rs` | `ESSENTIAL` | tool-name list | **T** |
| `Hook/Tool/Const.rs` | `EssentialTools` | tool-name list | **T** |
| `PollWorker/Should/Command/Fn.rs` | `SlowCommandPrefixes` | command prefixes | **T** |
| `Proxy/Analysis/Classify/FirstLine/Fn.rs` | `Rules` | 3 first-line rules | **T** |
| `Proxy/Upstream/Headers/Fn.rs` | `Skip` | forwarded-header skip list | **T** |
| `Flow/Signature/Const.rs` | `VolatileKeys` | signature-volatile keys | **T** |
| `Preview/Text/EnvelopeJSON.rs` | `Guard` | envelope guard words | **T** |
| `Guard/Loopback/Fn.rs` | `AllowedLoopbackHosts` | `["localhost", "127.0.0.1", ...]` | **P** (security) |
| `Resolve/Parse/Prefix.rs` · `Suffix.rs` | `Const` | `"<<<CCR "` · `">>>"` | **P** |
| `ChainSplit/Const.rs` | `SegMarker` | `"__APHRODITE_SEG__"` | **P** |
| `Hook/Live/Const.rs` | `LiveMetaSuffix` · `LiveChunkSuffix` | `" live meta"` · `" live chunk "` | **P** |

### 5.2 Thresholds

| Path | Const | Value | Class |
| --- | --- | --- | --- |
| `Proxy/Threshold/Const.rs` | `CacheCompressThreshold` | `8192` bytes | **T** |
| same | `TokenCompressThreshold` | `1024` bytes | **T** |
| same | `AutoExpandLimitDefault` | `102400` bytes | **T** |
| same | `InlineCCRThreshold` | `256` bytes | **T** |
| same | `InitialRatioEma` | `200` | **T** |
| `Stage2/Const.rs` | `MinimumStage2Size` | `80` bytes | **T** |

### 5.3 Caps and buffers

| Path | Const | Value | Class |
| --- | --- | --- | --- |
| `ABI/Guard/Content/Const.rs` | `MaximumContent` | `16 * 1024 * 1024` | **T** |
| `Proxy/Threshold/Const.rs` | `ResponseMaximumBodyBytes` | `64 MB` | **T** |
| same | `ResponseCacheMaximumBodyBytes` | `1 MB` | **T** |
| `Prefetch/Read/Const.rs` | `MaximumPrefetchSize` | `10 MB` | **T** |
| `Tool/Read/Const.rs` | `MAX_PATH_READ` | `10 MB` | **T** |
| `State/Const.rs` | `InlineMaximum` | `500` | **T** |
| same | `DefaultInlineByteBudget` | `256 MB` | **T** |
| same | `SplitEventCap` · `ToolEventCap` · `RecentMarkersCap` · `ReferencedFilesCap` | `16` · `200` · `200` · `100` | **T** |
| same | `SplitAdaptMinimumEvents` | `4` | **T** |
| `PollWorker/Task/Const.rs` | `MaximumBgTasks` · `StaleTurnAge` | `4` · `8` | **T** |
| `Session/Turn/Fn.rs` | `ConvIndexCap` | `50` | **T** |
| `Proxy/Metadata/JSON/Fn.rs` | `KeysLimit` | `10` | **T** |
| `Hook/Live/Const.rs` | `LiveChunkSize` | `1024` | **T** |
| `Proxy/State/Method/Record/Fn.rs` | `BucketLimits` | `[1e3, 1e4, 1e5, 1e6]` | **T** |

### 5.4 Preview and extraction

| Path | Const | Value | Class |
| --- | --- | --- | --- |
| `Preview/State/Maximum.rs` | `PreviewMaximumChars` | runtime atomic, default `120` | **T** |
| `StructExtract/MaximumSignatureLength.rs` | `Const` | `60` | **T** |
| `StructExtract/MaximumParametersLength.rs` | `Const` | `35` | **T** |
| `StructExtract/Budget.rs` | `Const` | `300` | **T** |
| `StructExtract/Detect.rs` | `AutoDetectScanLines` | `60` | **T** |
| `Directive/Model.rs` | `MaximumDirectiveChars` · `MaximumCombinedChars` | `2000` · `4000` | **T** |

### 5.5 Internal (stays constant)

`Proxy/Cache/Hash/Fn.rs` `FnvOffset`/`FnvPrime`; `Flow/Hash/Const.rs` `FnvOffset`/`FnvPrime`;
`Debug/Count/Const.rs` `Counters`; `Debug/Timing/Const.rs` `TimingSum`/`TimingN`;
`Debug/Level/Const.rs` `Level`; `Marker/Format/Const.rs` `MarkerFormat`/`MarkerHint`;
`ABI/Handle/Registry/Const.rs` `Handles`; every `OnceLock`/`Mutex` guard static; every fixture
constant under `*/Test*`.

A tunable FNV prime is not a parameter, it is a cache-key corruption waiting to be configured.

### 5.6 Content, not parameters

`Flow/Prompt/Const.rs` `FallbackPrompts` / `ShippedSessionInject`, and
`Flow/Retrieval/Guidance/Fn.rs` `RetrievalGuidanceStandard` / `RetrievalGuidanceVerbose`, are
**prose**. They belong in `.md` files under `crates/aphrodite/templates/` and are `include_str!`'d -
editable without a rebuild of the surrounding logic, and reviewable as text. They are not scalar
parameters and must not go in the TOML.

## 6. The benchmark harness

A parameter surface with no measurement is a configuration file nobody can tune. The harness is what
turns "edit → benchmark → see" into a number.

```
aphrodite bench --corpus <dir> --parameters <toml> --out <json>
aphrodite bench compare <a.json> <b.json>
```

- **Corpus**: real captured tool outputs, one file per case, named by content type
  (`cargo-test.txt`, `git-log.txt`, `ripgrep.txt`, `ls-long.txt`, ...). The fixtures already inside
  `Preview/Test/` are the seed; the corpus grows by capturing live sessions.
- **Metrics per case**: bytes in, bytes out, compression ratio, preview fidelity (does the preview
  still name the failing test / the changed file / the error line), and wall time. Fidelity is the
  one that matters: a ratio bought by a misleading preview is a regression, and the preview contract
  in `AGENTS.md` says so.
- **Score**: a single comparable number plus its component vector. A change is accepted when the
  score improves and no fidelity check regresses.
- **Determinism**: the corpus is fixed, the clock is injected, and the run prints the resolved
  parameter set with every result. A benchmark that does not record its own configuration measures
  nothing.

## 7. Generated cases - and the honest limit

The request: derive test cases from the parameter space and flood the pipeline with them.

That works, with one correction. **A full Cartesian product is not usable.** Ten parameters with
five values each is ~10 million cases. The standard answer is **pairwise (2-way) covering arrays**:
a set of cases in which every pair of parameter values appears at least once - typically a few dozen
for that same space, and empirically where most defects live. A pairwise sampler is ~100 lines and
needs no dependency.

The second correction, which matters more: **these are benchmark cases, not assertions.** A
generated case says "with these parameters, this input produced this ratio and this preview"; it
does not say "this output is correct". The project's standing position is that correctness is proven
by building and by reading, not by an assertion layer - and a generated assertion that encodes
today's output as truth locks in today's bugs. The generated corpus measures; the reviewer decides.

Concretely:

```
aphrodite bench generate --space parameters.space.toml --strategy pairwise --out cases/
```

where the space file names each parameter's candidate values. Each generated case is a directory:
the parameters, the input, and the expected *fidelity predicates* (not expected bytes).

## 8. Migration phases

Each phase is independently shippable and independently revertable.

| Phase | Deliverable | Acceptance |
| --- | --- | --- |
| **P1** | `parameters.toml` + typed `Parameters` struct + generator; every value still compiled in, nothing reads the file yet | binary behaviour byte-identical; the struct's fields equal the constants they replace |
| **P2** | Layer 1 wired: boot parses the embedded TOML into `Parameters`, stages receive it | same outputs as P1 on the corpus |
| **P3** | Layer 2 wired: `[parameters]` in `aphrodite.toml` overrides, with per-key warnings on bad values | a benchmark run at defaults and at an overridden value differs only in the overridden behaviour |
| **P4** | Layer 3 (env) + Layer 4 (ABI/MCP override) | an override via each layer changes the same observable |
| **P5** | Benchmark harness + corpus + score | two runs on the same parameters score identically |
| **P6** | Pairwise case generator | N cases generated from the space file; each runs and reports |

A phase that changes an output at default parameters is a bug in that phase, not a finding.

## 9. What this must not do

- **Must not let a parameter break stored data.** Marker grammar, segment marker and cache-key fields
  stay protocol constants. A test asserts they are not reachable from the parameters struct.
- **Must not make the dylib depend on a file existing.** Layer 1 is embedded; a missing user file is
  inherit, not failure.
- **Must not resolve parameters per call.** One resolution at boot; stages take the value.
- **Must not silently accept a bad override.** Warn with the key, the value and the fallback used.
- **Must not turn a benchmark into a gate.** It reports; a human accepts.

## Claim-to-test

| Claim | Probe | Pass condition |
| --- | --- | --- |
| No tuning constant remains hardcoded | `grep -rnE 'const [A-Z][A-Za-z]*\s*:\s*(usize\|u64\|&\[&str\])' crates/*/Source` | every hit is classed **P** or **I** in this document |
| The catalogue matches the defaults | the generator's `--check` mode | exit 0 |
| Protocol values are not configurable | a test that the parameters struct has no field feeding the marker grammar | compile error if one is added |
| A bad override degrades | set `threshold.cache.compress = "abc"` in the user toml, boot | one warning naming the key; pipeline runs at the default |
| Absent is not zero | remove a limit key, boot | the limit takes the embedded default, not `0` |
| A parameter change is measurable | two `bench` runs differing in one key | scores differ; the run prints the resolved parameters |
| Pairwise generation is bounded | `bench generate` on a 10×5 space | tens of cases, not millions; every pair covered |
