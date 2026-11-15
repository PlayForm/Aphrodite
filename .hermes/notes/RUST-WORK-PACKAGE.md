# Rust rewrite work package

## 1. Preamble

This is the execution map for the phase-4 code rewrite. It ties three documents to the cards that do
the work: `.hermes/notes/RUST-FUNCTIONAL-AUDIT.md` supplies the counts and the defect register,
`.hermes/notes/RUST-STYLE-FUNCTIONAL.md` supplies the rule and the forbidden shapes,
`.hermes/notes/RUST-REWRITE-PLAYBOOK.md` supplies the recipes with a real site for each, and
`.hermes/notes/HOUSE-STYLE-REFACTOR.md` supplies the layout contract (one export per file, PascalCase
modules, `mod.rs` facades that declare only). This document adds no rule of its own. Where a package
section names a recipe or a shape, it cites that document's section and does not restate it.

Measured at HEAD `a98bd3b7` on branch `Development`. Every count below was taken from a
`git archive HEAD` export of the two `Source` trees, not from the working tree, because a peer card's
uncommitted edit moves a number mid-run. Size is reproducible with:

```console
find <prefix> -name '*.rs' -print0 | xargs -0 cat | wc -l
find <prefix> -name '*.rs' | wc -l
```

A production count excludes any path containing `/Test/` and any `Test*.rs`; the difference between
the total and the production figure is the test tree, and a style pass reaches both. Where the
audit's headline total and its own per-module table disagree by 40 lines in `aphrodite-hermes`
(7,062 against 7,022), the module table is what the two commands above reproduce, and that is the
number used here.

**The standing rule of this programme: a card edits only the paths it owns.** A change outside them
is a defect even if it looks correct, because the other paths belong to a card that is editing them
at the same time and neither card can see the other's intent. There is one narrow exception and it is
stated in §4: a crate-level attribute change is a shared mutation and is sequenced last.

## 2. Package sections

Each section below carries the same six fields. `Owned paths` is the exact prefix set and the
prohibition that goes with it; `Size` is the audit's file and line count; `Recipes that apply` lists
the playbook recipes that have a site under these paths, by number, name and `path:line`;
`Defects in this package` carries the register's rows for these paths, with a `BEHAVIOUR` fix named
separately from the style pass and fixed first; `Acceptance` is the command ladder in order; `Risk`
is what a change here can break in a consumer outside the package, and what the card must therefore
report.

`vendor/**` is excluded from every package. `Maintain/**` and `plugins/**` are excluded.

## E1 - ABI exports, guard, diagnostics, state

**Owned paths** - `crates/aphrodite/Source/ABI/**`, `crates/aphrodite/Source/Guard/**`,
`crates/aphrodite/Source/Debug/**`, `crates/aphrodite/Source/State/**`,
`crates/aphrodite/Source/Logging/**`. A change outside these prefixes is a defect even if it looks
correct.

**Size** - 131 production files / 3,344 lines; 11 test files / 846 lines; 142 files / 4,190 lines in
total. By module: `ABI` 32 files / 1,311, `Guard` 11 / 229, `Debug` 51 / 766, `State` 27 / 854,
`Logging` 10 / 184.

**Recipes that apply** - nine, all from the playbook's own site list:

- 1.1 Ordered rejection rules stay guards - `ABI/Guard.rs:94`
- 1.2 A validator lifted into `Reject::Fn` and `?` - `ABI/Guard.rs:94`
- 1.3 Nullable pointer becomes `Option<NonNull<_>>` - `ABI/Guard.rs:74`
- 1.6 `map_err(...)?` instead of a match with a continuing `Ok` arm - `Guard/Bearer/Middleware.rs:40`
- 3.15 A configuration setter parses before the lock - `ABI/ConfigurationSet.rs:29`
- 3.16 A reload becomes a `ReloadAssessment` plus a `DiagnosticUpdate` - `ABI/Reload.rs:68`
- 4.6 A handle table becomes a `HandleRegistry` - `ABI/Handle/Table.rs:32`
- 5.4 One `ExportJSON` adapter owns panic interception, conversion and transfer - `ABI/ConfigurationSet.rs:48`
- 5.6 An owned prepared command replaces a `RefCell` in the ABI dispatch - `ABI/Dispatch.rs:71`

1.2 and 5.4 introduce leaves under `Guard/` and `ABI/` that do not exist yet; the naming for the
first is worked in the style guide §8 (`Reject`), so this card does not invent a name for it.

**Defects in this package** - none. No row of the audit's register has a path under these prefixes.
The `Debug/` here is `aphrodite`'s own diagnostics module; D1 is the `aphrodite-hermes` chain and
belongs to E9.

**Acceptance** - in this order, reporting the real exit code of each:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
python3 Maintain/check_ffi_contract.py
```

The last command is in this package's ladder and no other's: this is the card that touches the
exported symbols, and `Maintain/check_ffi_contract.py` is the command that settles whether the C ABI
still matches what `aphrodite-hermes`, the generated ctypes bindings and the plugin call.

**Risk** - four consumer surfaces live under these prefixes and none of them is internal:

- **24 `pub extern "C" fn aphrodite_*` symbols** under `Source/ABI/`. The symbol name is the ABI
  contract (cbindgen generates `_bindings.py` from it and `Maintain/check_ffi_contract.py` asserts
  it), so a rename, an arity change or a pointer-type change breaks the plugin's loader.
- **The 13 host-call arms** in the `match name.as_ref()` at `ABI/Dispatch.rs:81` (14 spellings, since
  one arm accepts two) - `on_session_start` | `session_start`, `transform_tool_result`,
  `transform_terminal_output`, `pre_llm_call`, `post_llm_call`, `catalog`, `stats`, `directive`,
  `search`, `diff`, `files`, `classify`, `prefetch`. These are the strings Hermes passes in; converting
  the dispatch to an enum is right, but the parse of those strings stays byte-identical.
- **The configuration key strings** in `ABI/ConfigurationSet.rs` (4 arms) and
  `ABI/ConfigurationGet.rs` (5 arms), which are the same names the TOML and `APHRODITE_*` layer use.
- **The JSON error envelope.** `extern "C"` never returns `anyhow`; the boundary renders
  `format!("{e:#}")` into the payload a caller parses, so every error string is a wire string.

Report each of the four as a `Consumer surface` row in the handoff: symbol, key string, call name or
error string, with before and after. A `pub` path this crate exposes to `aphrodite-hermes` is a fifth
surface; see §4.

## E2 - the proxy pipeline

**Owned paths** - `crates/aphrodite/Source/Proxy/**`, including `Proxy/Test/**`. A change outside
this prefix is a defect even if it looks correct.

**Size** - 134 production files / 4,452 lines; 38 test files / 1,774 lines; 172 files / 6,226 lines
in total. The densest package in the crate: 150 production findings in the audit's counts, and
`Proxy/Handle/Proxy/Respond/Fn.rs:30` is 337 lines, the largest function in either crate.

**Recipes that apply** - thirteen:

- 1.5 A cached value resolved through `filter` then `map` - `Proxy/Cache/Get/Fn.rs:21`
- 2.3 Bounded async accumulation becomes a fallible fold - `Proxy/Handle/Proxy/Forward/Fn.rs:47`
- 3.1 Five independent flags become one route enum - `Proxy/Handle/Proxy/Respond/Fn.rs:30`
- 3.4 `Option<key>` plus `Option<response>` becomes `CacheLookup` - `Proxy/Handle/Proxy/Route/Fn.rs:84`
- 3.6 A `(Result, bool)` becomes `Result<_, ForwardFailure>` - `Proxy/Handle/Proxy/Forward/Fn.rs:24`
- 3.7 A retry loop becomes a `RetryDecision` - `Proxy/Handle/Proxy/Forward/Fn.rs:95`
- 3.19 A supplied key versus a digest becomes a `ContentLocation` - `Proxy/CCR/Create/Handle/Fn.rs:43`
- 3.20 `&mut Vec<String>` becomes a typed `MetadataField` - `Proxy/Metadata/Build/Fn.rs:3`
- 4.8 A double-serialised body becomes `FinalBody` over `Bytes` - `Proxy/Handle/Proxy/Respond/Fn.rs:204`
- 6.3 One `ContentAnalysis` serves both the preview and the metadata renderer - `Proxy/Metadata/Fn.rs:17`
- 6.5 `floor_char_boundary` replaces a hand-written boundary walk - `Proxy/Body/Truncate/Fn.rs:12`
- 7.3 `Bytes` for a body that is read twice - `Proxy/Handle/Proxy/Respond/Fn.rs:297`
- 7.4 Serialise once, then share - `Proxy/Handle/Proxy/Respond/Fn.rs:190`

3.20 is one recipe over thirteen call sites (the metadata extractors, each
`Fn(Content:&str, ..., Parts:&mut Vec<String>)`), so it is one representation decision applied
thirteen times, not thirteen independent edits.

**Defects in this package** - one, and it is a `BEHAVIOUR` fix:

- **D3, `BEHAVIOUR`, fixed first, as its own named change.** `Proxy/Body/Stream/Fn.rs:27-28` accepts
  the byte run `"stream":true` anywhere in the body, while `Proxy/Cache/Key/Fn.rs:20` parses the
  top-level field. Two predicates, one body: `{"model":"m","stream":false,"meta":{"stream": true}}`
  is streaming to the fast path and non-streaming to the cache key. Consequence: a request is
  forwarded on the streaming client, which carries no total timeout, and is also cacheable, so a
  buffered body can be stored and later replayed for a request the proxy classified as streaming.
  No playbook recipe covers this one - the fix is that the decision is taken once, from the parsed
  top-level field, and the style guide §2 is the rule that names it. It is not a style pass and must
  not be described as one: the handoff says which predicate now decides and what a body with a nested
  `stream` key does after the change.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
```

D3's fix is settled by the existing proxy tests under `Proxy/Test/**` extended in place; this epic
adds no test harness (§4).

**Risk** - this package holds the wire:

- **Serde field names on wire types.** `Proxy/Relay/Request/Struct.rs` and
  `Proxy/CCR/Notification/Struct.rs` are serialised onto the proxy wire, as are the configuration
  tables; the style guide §9 forbids renaming a field there and forbids tidying a `#[serde(rename)]`
  that exists to keep a wire name.
- **The cache key.** `Proxy/Cache/Key/Fn.rs` computes the key a stored entry is found under, so a
  change to how it is derived silently invalidates or aliases every entry already in the store. A
  change to the key's derivation is a behaviour change and is reported as one.
- **The stats keys.** `Proxy/State/Method/Stats/**` produces the map that `Boot/Route/Metrics.rs`
  renders as `aphrodite_*` metric lines (E6 owns the renderer, this package owns the keys), so a
  renamed key renders as a missing metric rather than as a compile error.
- **Threshold and cap field names** in `Proxy/Threshold/Struct.rs` and `Proxy/State/Struct.rs`, which
  are the names the configuration layer sets.

Report each as a `Consumer surface` row with before and after, and name any key-derivation change as
a `BEHAVIOUR` change with the input that now maps differently.

## E3 - hook plans, chain split, session, prefetch

**Owned paths** - `crates/aphrodite/Source/Hook/**`, `crates/aphrodite/Source/ChainSplit/**`,
`crates/aphrodite/Source/Session/**`, `crates/aphrodite/Source/Prefetch/**`. A change outside these
prefixes is a defect even if it looks correct.

**Size** - 107 production files / 2,658 lines; 17 test files / 1,379 lines; 124 files / 4,037 lines
in total. By module: `Hook` 60 files / 1,363, `ChainSplit` 13 / 311, `Session` 13 / 454, `Prefetch`
21 / 530.

**Recipes that apply** - eight:

- 3.5 `DidCompress` plus a mutated JSON becomes `RewriteOutcome` - `Hook/ToolResult/Inner/Fn.rs:43`
- 3.8 A terminal decision tree becomes a `TerminalPlan` - `Hook/Terminal/ChainSplit/Fn.rs:26`
- 3.13 A tool result becomes a `ToolObservation` - `Hook/ToolResult/Inner/Fn.rs:25`
- 3.14 Path extraction carries its provenance - `Hook/Path/Fn.rs:10`
- 4.1 Storage evidence before marker rendering - `Hook/Live/Container.rs:38`
- 5.1 Prepare the segments, then apply them - `Hook/Terminal/ChainSplit/Fn.rs:44`
- 6.4 A UTF-8 chunk iterator replaces a mutable collector - `Hook/Live/Chunk/Fn.rs:11`
- 7.1 `Cow` for an optional text transform - `Hook/Terminal/ChainSplit/Fn.rs:60`

`Hook/ToolResult/Inner/Fn.rs:25` is 136 lines and `Hook/Terminal/ChainSplit/Fn.rs:26` is 83; both
split along the stage boundaries the recipes name, and 3.5 and 3.13 are the two halves of the first.

**Defects in this package** - none of the register's seven rows falls under these prefixes.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
```

**Risk** - this package produces the text the model reads and the payload Hermes replaces a tool
result with:

- **The hook return envelope.** `transform_tool_result` and `transform_terminal_output` return a JSON
  object whose shape Hermes parses; an empty object means "no replacement". A refactor that changes
  which key is populated changes whether the model sees compressed content or the original.
- **The marker text** rendered from `Hook/Live/Container.rs` - the `<<<CCR:hash|type|size>>>` string
  is the retrieval vocabulary, so its format is a contract with the model and with `Marker/`'s parser.
- **The session archive and index layout** under `Session/Archive/**` and `Session/Index/**`, which is
  read back by `Session/Catalog` (same package) and by `Catalog/` (E7).
- **The prefetch status keys** under `Prefetch/Status/**`, which the `prefetch` host call returns.

Report a changed key, marker format or envelope field as a `Consumer surface` row; report a changed
ordering between the telemetry record and the compress decision as a `BEHAVIOUR` change, because the
audit's split of `Hook/ToolResult/Inner/Fn.rs:25` says the telemetry record must run first.

## E4 - poll worker and flow windows

**Owned paths** - `crates/aphrodite/Source/PollWorker/**`, `crates/aphrodite/Source/Flow/**`. A
change outside these prefixes is a defect even if it looks correct.

**Size** - 70 production files / 1,616 lines; 21 test files / 1,187 lines; 91 files / 2,803 lines in
total. By module: `PollWorker` 23 files / 672, `Flow` 47 / 944.

**Recipes that apply** - six, two of them `BEHAVIOUR`:

- 3.9 A poll update becomes `PollEvent` plus `TaskTransition` - **BEHAVIOUR** - `PollWorker/Update/Fn.rs:97`
- 3.10 Expiry becomes a `RetentionDecision` with a report - `PollWorker/Expire/Fn.rs:17`
- 3.11 A nudge loop becomes a `NudgePlan` - `PollWorker/Check/Fn.rs:21`
- 4.2 The poll output cap becomes `BoundedOutput` - **BEHAVIOUR** - `PollWorker/Update/Fn.rs:63`
- 4.7 A task vector becomes a `TaskRegistry` - `PollWorker/Task/Struct.rs:14`
- 7.2 `std::mem::take` for completion ownership - `PollWorker/Update/Fn.rs:121`

**Defects in this package** - three, all in the same 121-line function (`PollWorker/Update/Fn.rs:22`,
which the audit's size table already splits into an accumulate phase and a complete phase). They are
fixed first, in this order, one named change each, before any style work in the file:

1. **D6, `BEHAVIOUR`.** `PollWorker/Update/Fn.rs:97-126` parses one poll result's exit code and
   applies it to every task whose status is `Running`. Consequence: the first poll carrying an exit
   code finishes every running task, so a second still-running build is reported done, its
   accumulator is cleared and its marker records the other task's exit code. `MaximumBgTasks = 4`
   (`PollWorker/Task/Const.rs:11`) makes more than one running task the supported case, not the
   exotic one.
2. **D4, `BEHAVIOUR`.** `PollWorker/Update/Fn.rs:63-71` tests the cap before the mutation it guards,
   then appends a whole poll payload whose size is unbounded. Consequence: the accumulator exceeds the
   stated 256KB cap by up to one poll payload, and `LiveBytesArrived` reports the over-cap length as
   the bytes arrived. Fixed by recipe 4.2, which moves the bound to the type that owns it.
3. **D7, a cost defect, not a correctness break.** `PollWorker/Update/Fn.rs:102` clones each running
   task's 256KB-capped accumulator inside a `filter_map` and clears the original at `:121`.
   Consequence: up to 1MB is copied per poll to move a string that is then discarded; recipe 7.2
   replaces the copy with a take.

D4 and D6 share lines 63-126, so the two fixes are one restructure with two named behaviour changes
and two separate probes, not one "fix the poll worker" change.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
```

`PollWorker/Test/` already carries `Update/`, `Status.rs`, `Marker.rs`, `Should.rs`, `Insert.rs`,
`Expire.rs` and `Check.rs`; each defect's fix extends the test at its own site in place. No new
harness.

**Risk** - the poll surface is what Hermes polls and what the model sees:

- **The poll event JSON** and the status names it carries, which the `stats` and hook paths read.
- **The marker recorded per completed task**, which is the retrieval handle for that task's output -
  a marker whose hash or size is computed differently resolves to the wrong byte count.
- **`MaximumBgTasks = 4`** at `PollWorker/Task/Const.rs:11`, the constant the concurrency of the whole
  mechanism rests on.
- **`LiveBytesArrived`**, a metric name emitted from this package and read by the metrics route.

D6 and D7 change what a caller observes under concurrency, so the handoff states the interleaving it
was verified against, not only that the tests pass.

## E5 - preview, stage-2 reducers, struct extract

**Owned paths** - `crates/aphrodite/Source/Preview/**`, `crates/aphrodite/Source/Stage2/**`,
`crates/aphrodite/Source/StructExtract/**`. A change outside these prefixes is a defect even if it
looks correct.

**Size** - 121 production files / 4,184 lines; 24 test files / 1,358 lines; 145 files / 5,542 lines
in total. By module: `Preview` 86 files / 2,778, `Stage2` 21 / 536, `StructExtract` 14 / 870. The
highest finding density in the crate: 61 production findings over 870 lines in `StructExtract`, and
`StructExtract/Rust.rs:26` is 207 lines.

**Recipes that apply** - nine, one of them `BEHAVIOUR`:

- 1.4 Eager alternative tuple becomes an `or_else`/`find_map` chain - `Preview/Text/FirstMeaningful.rs:24`
- 2.2 Budget exhaustion becomes `ControlFlow::Break` - `StructExtract/Rust.rs:32`
- 3.21 A regex-string protocol becomes a `SignatureMatcher` - `Stage2/Reduce/Code/Fn.rs:21`
- 3.22 Declaration recognition becomes a `Recognition` enum - `StructExtract/Rust.rs:42`
- 4.3 A reducer sample becomes `CountedSample<T, N>` - `Stage2/Reduce/Build/Fn.rs:16`
- 4.4 An extraction budget becomes `ExtractionBudget` plus `BudgetPolicy` - `StructExtract/Rust.rs:26`
- 4.5 `limit == 0` becomes `PreviewLimit::{Unlimited, Characters(NonZeroUsize)}` - `Preview/Builder/Cap.rs:16`
- 6.1 Preview builders become `Parse` then `Summary` then `Render` - `Preview/Builder/Ls.rs:14`
- 6.2 A git log becomes a record-boundary state machine - **BEHAVIOUR** - `Preview/Builder/GitLog.rs:24`

**Defects in this package** - one, and it is a `BEHAVIOUR` fix:

- **D5, `BEHAVIOUR`, fixed first.** `Preview/Builder/GitLog.rs:24-34` rejects `commit ` lines instead
  of stopping at them, so the subject search continues into the next commit's block. Consequence: a
  commit with an empty message inherits the next commit's subject, and since a log preview's
  first/last endpoints are the whole preview, the preview names the wrong change. Recipe 6.2 is the
  fix and is the only place in this package where the change is described as a behaviour change; the
  other eight recipes are style passes and must not be.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
```

`Preview/Builder/Test.rs` exists (a 130-line table of runner output shapes) and `StructExtract/Test/`
and `Stage2/Test/` exist; D5's fixture is added to the git-log test in place.

**Risk** - this package owns what the model is shown and what is stored as the compressed payload:

- **The preview text.** The project's standing rule is that the preview the model sees is the final,
  honest representation of the stored payload, so a preview that renders differently after a style
  pass is a regression even when the stored bytes are identical.
- **`preview_max_chars`** - `APHRODITE_PREVIEW_MAX_CHARS` over TOML over a default of 120, with an
  absent key meaning unlimited. The precedence is a behaviour, and 4.5 changes the representation of
  the "unlimited" case, so the handoff states that the three-way precedence still resolves the same.
- **`total_count`**, which is a real count in the preview, not an estimate.
- **The stage-2 reducer output**, which becomes the content stored in the CCR database. A reducer that
  samples differently changes what a later retrieval returns for an entry already stored.
- **The declaration matchers** in `StructExtract/`, whose output feeds the metadata renderer (E2's
  3.20/6.3) - a matcher that recognises one construct fewer silently shortens every metadata block.

Report a changed preview line, a changed `total_count` or a changed reducer sample as a
`Consumer surface` row with before and after.

## E6 - boot, configuration, setup, home

**Owned paths** - `crates/aphrodite/Source/Boot/**`,
`crates/aphrodite/Source/ConfigurationLoader/**`, `crates/aphrodite/Source/Configuration/**`,
`crates/aphrodite/Source/Setup/**`, `crates/aphrodite/Source/Home/**`. A change outside these
prefixes is a defect even if it looks correct.

**Size** - 143 production files / 4,695 lines; 26 test files / 1,353 lines; 169 files / 6,048 lines
in total. By module: `Boot` 30 files / 1,429, `ConfigurationLoader` 34 / 963, `Configuration` 33 /
817, `Setup` 22 / 1,163, `Home` 24 / 323. `Boot/Route/Metrics.rs:42` is 201 lines and
`Boot/Listen/Watcher.rs:26` is 91.

**Recipes that apply** - six:

- 2.4 A debounce drain becomes `from_fn(|| Rx.try_recv().ok())` - `Boot/Listen/Watcher.rs:80`
- 2.5 Channel state matched directly, `None => break` - `Boot/Listen/Watcher.rs:74`
- 3.2 A `(mode, name)` tuple becomes `ConfigurationSource` - `Configuration/Proxy/Multi/Resolve/Fn.rs:47`
- 3.18 Registration returns an outcome instead of erasing a warning into `Ok` - `Setup/Write/Register.rs:24`
- 5.2 `BindOne` extracted from the stream pipeline - `Boot/Listen/Bind.rs:34`
- 5.5 The watcher split into event source, debounce controller and session apply - `Boot/Listen/Watcher.rs:41`

5.2 costs a restructure rather than an edit - `Boot/Listen/Bind.rs` becomes `Bind/mod.rs` plus
`Bind/One/Fn.rs` and `Bind/All/Fn.rs`, and `Boot/Run.rs` repoints the call. The style guide §8 works
that path; this card follows it and does not invent a third name.

**Defects in this package** - one register row, and it is the one that is **not a fix**:

- **D2, REFUTED.** `Boot/Listen/Watcher.rs:74` reads `let Received = Rx.recv().await.is_some()` with a
  `false => {}` arm. The audit's reading of the scope settles it: `Tx` is owned by the `notify`
  closure, which is stored in `Watcher`, a local of the same async block as the loop, so no sender can
  be dropped while the loop runs and the arm is dead at HEAD. Recipe 2.5 still replaces it, because
  the arm becomes a 100% CPU busy loop the moment the sender is dropped or moved into a shorter-lived
  scope. The handoff says "removed dead code", not "fixed a spin" - the two are different reports and
  only one of them is true.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
```

**Risk** - this package owns every name a user or an operator types:

- **The metric names** rendered by `Boot/Route/Metrics.rs` - `aphrodite_requests_total`,
  `aphrodite_requests_compressed_total`, `aphrodite_tokens_saved_total`, `aphrodite_ccr_hits_total`,
  `aphrodite_ccr_misses_total`, `aphrodite_ccr_created_total`, `aphrodite_tool_relay_calls_total`,
  `aphrodite_cache_hits_total` and the families beside them. A renamed metric does not fail to
  compile; it disappears from a dashboard.
- **The `APHRODITE_*` environment names and the TOML keys**, which are the configuration contract -
  `APHRODITE_PREVIEW_MAX_CHARS`, `APHRODITE_CCR_TTL`, `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS` and the
  rest. The precedence the configuration layer resolves (environment over TOML over default) is a
  behaviour, and 3.2 changes the representation of that resolution without changing its order.
- **The listener ports** (the shipped `aphrodite.toml` carries the cache proxy on 9797 and the token
  proxy on 9798) and the bind sequence 5.2 restructures.
- **The install plan.** `Setup/Write/**` writes into the runtime home and registers the plugin, so a
  reordered step is an install behaviour change even when every step still runs.

Report each renamed metric, environment name, TOML key and port as a `Consumer surface` row, and
report any change to the install step order as a `BEHAVIOUR` change.

## E7 - retrieval, directives, templates, scripting, catalog

**Owned paths** - `crates/aphrodite/Source/Directive/**`, `crates/aphrodite/Source/Template/**`,
`crates/aphrodite/Source/Scripting/**`, `crates/aphrodite/Source/Resolve/**`,
`crates/aphrodite/Source/Catalog/**`, `crates/aphrodite/Source/Retrieve/**`. A change outside these
prefixes is a defect even if it looks correct.

**Size** - 102 production files / 2,734 lines; 19 test files / 1,737 lines; 121 files / 4,471 lines
in total. By module: `Directive` 6 files / 388, `Template` 40 / 661, `Scripting` 19 / 402, `Resolve`
12 / 500, `Catalog` 7 / 211, `Retrieve` 18 / 572. `Retrieve/Handle/Fn.rs:26` is 198 lines and
`Directive/Action.rs:25` is 88.

**Recipes that apply** - two, the fewest of any package:

- 3.3 Absent and empty collapse into `RetrieveSource` - `Retrieve/Handle/Fn.rs:27`
- 7.5 Memory-bounded line sampling instead of collecting every line - `Retrieve/Page/Fn.rs:24`

Two recipes over 2,734 lines is the whole reason this package runs alone in the last wave: the
remaining work here is the per-module atomization the layout contract already implies (198 lines in
one file, 88 in another), not a catalogue of shapes. A card that finds a third recipe's shape here
reports it rather than inventing a site for it.

**Defects in this package** - none of the register's seven rows falls under these prefixes.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite --all-targets
cargo test -p aphrodite
cargo clippy -p aphrodite -- -D warnings
cargo fmt --all -- --check
```

**Risk** - this package owns the retrieval vocabulary the model is taught:

- **The catalog text** produced by `Catalog/Build/Fn.rs:12` is what the model reads to decide what to
  retrieve, so its grouping and its size accounting are the retrieval contract.
- **The builtin directive names.** `Directive/Builtin/**` carries the `.md` files whose filenames are
  the directive identifiers (`focus`, `foresight`, `ccr-handling` and the rest) surfaced to the model
  and to `aphrodite_directive("add", ...)`. The directory is PascalCase; the data filenames are keys
  and do not get renamed (HOUSE-STYLE-REFACTOR's deviations section says so explicitly).
- **The retrieve page window** in `Retrieve/Page/**`: a marker's `size` is the number of bytes a
  retrieval re-adds to context, so a changed window changes what a retrieval costs and returns.
- **Template variable names** in `Template/Vars/**`, which appear in directive text a user can
  override.

Report a changed catalog line, directive identifier, page window or template variable name as a
`Consumer surface` row.

## E9 - the bridge crate

**Owned paths** - `crates/aphrodite-hermes/Source/**`, the whole crate. A change outside this prefix
is a defect even if it looks correct. `Source/Library.rs` sits inside this prefix and carries the
crate-level attribute change named in §5.

**Size** - 200 production files / 4,837 lines; 50 test files / 2,185 lines; 250 files / 7,022 lines
in total. By module: `Tool` 44 files / 1,144, `Hook` 31 / 865, `Debug` 41 / 862, `Schema` 35 / 652,
`Session` 13 / 375, `Directive` 9 / 279, `ABI` 11 / 228, `Configuration` 7 / 166, `Health` 5 / 90,
`Guard` 2 / 69, `Bin` 1 / 32, `(root)` 1 / 75. Five of the twelve modules are clean - `Schema`,
`Health`, `Guard`, `Bin`, `(root)` carry zero production findings - so the working set is the other
seven, 4,837 production lines of it.

**Recipes that apply** - seven:

- 1.7 A `HashMap` built by `collect` instead of mutated - `Tool/Registry/Fn.rs:39`
- 1.8 `warnings` consumed at a merge boundary, not cloned - `Directive/Into/Fn.rs:35`
- 2.1 A bounded parent walk becomes `successors(...).take(17)` - `Debug/Chain/Fn.rs:38`
- 2.6 A `Result`-as-break sentinel deleted - `Session/Watcher/Fn.rs:70`
- 3.12 Backgrounding becomes a `BackgroundPlan` - `Hook/Call/Dispatch/PreTool/Fn.rs:44`
- 3.17 Tool dispatch prepares a command before it dispatches - `Tool/Dispatch/Fn.rs:33`
- 5.3 A guard released before a locking helper is called - `Debug/Session/Record/Fn.rs:52`

`Tool/Unwrap/Fn.rs:27` (134 lines) and `Hook/Call/Dispatch/PreTool/Fn.rs:21` (106 lines) are the
package's two size outliers and split along the arms 3.17 and 3.12 name.

**Defects in this package** - one, a `BEHAVIOUR` fix that is **already in the working tree**:

- **D1, `BEHAVIOUR`.** `Debug/Session/Record/Fn.rs:52` binds the `SESSION_PARENTS` guard to a named
  `let`, so it lives to the end of the function, and line 60 called `Chain::Fn::Fn(session)`, which
  locks the same `OnceLock<Mutex<HashMap>>` at `Debug/Chain/Fn.rs:20-23`. `std::sync::Mutex` is not
  reentrant, so the thread blocked on its own lock and never returned: the hook hung until Hermes
  abandoned the worker, and every later chain call blocked behind the mutex the stuck thread still
  held, so the session tree stopped resolving its root.

    The boundary fix is already applied in the tree at measurement time - `Record/Fn.rs` takes the guard
    in one scope, inserts the edge and calls `Chain::Fn::chain_in(&map, session)`, which takes the map
    by reference and locks nothing. This card builds on it and does not redo it. What it must finish is
    the naming half the style guide §6 requires: `Debug/Chain/Fn.rs` now carries **two** exports,
    `chain_in` at `:32` and `Fn` at `:58`, which the layout contract forbids in one leaf. The pure walk
    moves to `Debug/Chain/In/Fn.rs` as `Chain::In::Fn::Fn`, the locking wrapper keeps `Chain::Fn::Fn`,
    and `Record/Fn.rs` repoints to the new path. Recipe 5.3 documents that two-leaf shape; the second
    export's `snake_case` name is the same defect seen from the naming side.

**Acceptance** - in this order:

```sh
cargo check -p aphrodite-hermes --all-targets
cargo test -p aphrodite-hermes
cargo clippy -p aphrodite-hermes -- -D warnings
cargo fmt --all -- --check
```

**Risk** - this crate is the only thing Hermes talks to:

- **11 `pub extern "C" fn aphrodite_hermes_*` symbols** under `Source/ABI/`, the same contract class
  as E1's, but for the plugin's own entry points.
- **The tool envelope JSON** parsed from Hermes and the tool names registered back into it, including
  the `aphrodite_debug` family that is compiled in only under `debug_assertions`
  (`Tool/Registry/Fn.rs:31`).
- **The Schema JSON** under `Source/Schema/**` - `Schema` is one of the five clean modules, so it is
  not a style target and a change to a schema shape is out of scope for this card.
- **The `aphrodite::...` paths this crate imports from the sibling crate.** Every rename or move in
  E1-E7 that touches a path this crate names is a build break here, which is why §4 requires listing
  such a change in the handoff. This is also the reason a card that renames a `pub` path runs the
  sibling crate's `cargo check` once after its own gate is green.
- **The session registry keys** under `Source/Session/**`, shared with the hook path.

Report a changed symbol, envelope field, tool name or imported path as a `Consumer surface` row.

## V1 - verification

**Owned paths** - none during the four waves. At the end, two files and nothing else:
`crates/aphrodite/Source/Library.rs` and `crates/aphrodite-hermes/Source/Library.rs`, for the removal
of the two crate-level clippy allows the style guide §1 requires (see §5). A change outside those two
files is a defect even if it looks correct.

**Size** - no code of its own. The two root files are 2 files / 119 lines in `aphrodite` and 1 file /
75 lines in `aphrodite-hermes`, and the audit counts no findings against either.

**Recipes that apply** - none. V1 applies no recipe; it runs the ladder and settles the claims in §6.

**Defects in this package** - none. V1 is the card that closes the register, so it reports every row's
verdict rather than owning a row.

**Acceptance** - the only card that runs the full ladder, in this order:

```sh
cargo check -p aphrodite -p aphrodite-hermes --all-targets
cargo test -p aphrodite
cargo test -p aphrodite-hermes
cargo clippy -p aphrodite -p aphrodite-hermes --all-targets -- -D warnings
cargo fmt --all -- --check
python3 Maintain/check_ffi_contract.py
```

followed by the forbidden-shape greps and the per-package `if` probe of §6 and §7, each reported with
its number.

**Risk** - V1 performs the one write in this programme that is not file-disjoint: a crate-level
attribute is crate-wide state, so removing it changes the gate for every file in the crate at once.
That is why it runs after the last package rather than in wave 1, and why it reports the two removals
as its own named change. Everything else V1 does is read-only.

## 3. Ordering and why

The programme runs as four waves of two packages plus one lane that spans them:

| Wave | Packages | Why together                                              |
| ---- | -------- | --------------------------------------------------------- |
| 1    | E1 ‖ E2  | the ABI/entry surface and the proxy pipeline              |
| 2    | E3 ‖ E4  | the hook/session text path and the poll worker            |
| 3    | E5 ‖ E6  | preview/reducers/extract and boot/configuration/setup     |
| 4    | E7       | the smallest recipe count, and the one that can run alone |
| -    | E9       | alongside every wave, then V1 after the last one          |

**The reason is one Cargo workspace.** `Cargo.toml` declares
`members = ["crates/aphrodite", "crates/aphrodite-hermes"]`, so every `cargo check` in the programme
contends on the same `target/` directory and the same package-cache lock. Two workers in the same
crate therefore serialise on the build whether or not their files overlap. The pairs are chosen to be
**file-disjoint** - no prefix in one package contains a prefix of the other - so that contention
stays scheduling noise and never becomes a merge. E9 sits in the same workspace and takes the same
lock, but it owns no file another card touches, which is why it can run alongside every wave.

**The peer-attribution rule, stated once and applying to every card: an error in a path you do not
own is a sibling's in-flight edit, report it as peer-attributed, never edit around it.**

Two consequences follow from the wave structure and are not negotiable:

- A card that fails its gate on a file outside its own prefixes reports the failure with the path, the
  package it belongs to and the exact error, and stops. Editing around it would put two cards in one
  file and lose both.
- The crate-level attribute removal is sequenced **last**, in V1, because a crate-level allow removed
  before the passes land reddens every remaining package's `cargo clippy -- -D warnings` for code it
  does not own yet - 243 of the audit's 407 production no-op-arm hits have a boolean scrutinee, so the
  lint would fire across the whole crate at once. This is the single place where the plan's
  file-disjointness does not hold, and it is resolved by ordering rather than by ownership.

## 4. Cross-cutting rules every package obeys

- **A style pass changes no observable behaviour**: same error text, same JSON, same metric names, same
  ABI symbols, same ordering. A behaviour change is its own named fix, reported separately. The
  `BEHAVIOUR` items in this map are D1, D3, D4, D5 and D6, and the playbook's 3.9, 4.2 and 6.2 - each
  one is a separate entry in the handoff with the input that now behaves differently.
- **No `pub` path is renamed or moved without listing it in the handoff**, because
  `crates/aphrodite-hermes` consumes `aphrodite::...` paths and a rename there is a build break in a
  crate the card does not own. A card that renames a `pub` path runs
  `cargo check -p aphrodite-hermes --all-targets` once after its own gate is green and reports a
  failure in a file it does not own as peer-attributed.
- **A new module obeys the layout contract**: PascalCase directory, `mod.rs` declaring only, one
  export per leaf, item named by kind. No `pub use`. The directory carries the domain, the file
  carries the kind, the call site carries the whole path, and there is no alias - the style guide §8
  is the worked form of this rule.
- **The gate is `cargo check` / `cargo test` / `cargo clippy -- -D warnings` for the package**: no new
  gate script, no new test harness. Where a defect's fix needs a fixture, it extends the test file
  that already sits at that site. `Maintain/check_ffi_contract.py` is the one existing command outside
  the cargo ladder and it belongs to E1 and V1 only.
- **No git writes.** An external auto-committer sweeps the tree. A clean `git status` therefore does
  not mean a card committed anything, and a card never stages, commits, amends or restores.

## 5. Corrections to the dispatch map

The map was measured against the audit's module table. Every package's size reproduces exactly, with
one exception, and two Rust prefixes under `crates/aphrodite/Source` are in no package at all,
alongside one data directory that carries no Rust. Each item below states what the map says, what the
tree says, the evidence, and which card's body needs amending. Only the orchestrator can amend a
dispatched card, so these are requests, not actions.

### C1 - `Marker/` is in no package

- **What the map says**: eight packages covering `ABI`, `Guard`, `Debug`, `State`, `Logging`, `Proxy`,
  `Hook`, `ChainSplit`, `Session`, `Prefetch`, `PollWorker`, `Flow`, `Preview`, `Stage2`,
  `StructExtract`, `Boot`, `ConfigurationLoader`, `Configuration`, `Setup`, `Home`, `Directive`,
  `Template`, `Scripting`, `Resolve`, `Catalog`, `Retrieve`, and the whole `aphrodite-hermes` crate.
- **What the tree says**: `crates/aphrodite/Source/Marker/` exists and is in none of those prefixes.
- **Evidence**: 17 files / 964 lines (13 production / 527 plus 4 test / 437), 4 `if` occurrences by the
  §7 probe, and 12 production findings in the audit's module table - including
  `Marker/Format/Entry.rs:50` (`Safe = match ()` in expression position) and
  `Marker/Format/Bracket.rs:24` (`&Inner[..Colon]`, an unguarded byte slice). No playbook recipe has a
  site here, so it is a style pass with no catalogue entry.
- **Request**: add `crates/aphrodite/Source/Marker/**` to **E3**. E3's own note already says "marker
  assembly", and four of E3's files consume the module (`Hook/ToolResult/Inner/Fn.rs`,
  `Hook/Terminal/Emit/Fn.rs`, `Hook/Terminal/ChainSplit/Fn.rs`, `Hook/Live/Container/Fn.rs`), so the
  package that renders the marker is the one that should own its format. The alternative is **E1**,
  where `ABI/Compress.rs` also consumes it and the package is the lightest at 3,344 production lines;
  choose E1 only if keeping E3 at eight recipes matters more than locality, and note that the choice
  moves the work one wave either way (E1 wave 1, E3 wave 2).

### C2 - the crate root is in no package, and the allow removal has no owner

- **What the map says**: nothing about `crates/aphrodite/Source/Library.rs` or
  `crates/aphrodite/Source/Main.rs`.
- **What the tree says**: two files, 119 lines, zero findings in the audit's counts - a clean module,
  but not a no-op one: `Library.rs:43-44` carries `#![allow(clippy::match_bool)]` and
  `#![allow(clippy::single_match)]`, and the style guide §1 requires both to be removed as part of
  applying the style. The `aphrodite-hermes` counterpart is `Library.rs:49`
  (`#![allow(clippy::match_bool, clippy::single_match)]`, with the comment at `:48`), which sits
  inside E9's prefix and therefore already has an owner.
- **Evidence**: `grep -n 'allow' crates/aphrodite/Source/Library.rs` returns five attributes, two of
  which the style guide revokes; the audit's measurement 2 counts 407 production no-op-arm hits, 243
  of them with a boolean scrutinee, which is what those two allows are currently suppressing.
- **Request**: give the two `aphrodite` root files to **V1**, and require V1 to remove both allows in
  both crates as one named change after the last package has landed. Do not give them to a wave-1
  card: a crate-level allow removed at the start reddens every later package's clippy gate for code
  that package does not own, which converts the whole programme into peer-attributed failures. This is
  the reason V1 owns a write at all, and it should be stated in V1's body as its only write.

### C3 - E9's size is understated

- **What the map says**: `crates/aphrodite-hermes/Source/**`, `~5.9k` lines.
- **What the tree says**: 250 files / 7,022 lines, of which 200 files / 4,837 lines are production and
  50 files / 2,185 are test.
- **Evidence**: the two commands in §1, both of which return 7,022 for this tree; the audit's
  per-module table sums to the same 7,022 and the same 4,837 production lines. Every other package's
  map figure reproduces exactly as its production-plus-test total (E1 4,190, E2 6,226, E3 4,037,
  E4 2,803, E5 5,542, E6 6,048, E7 4,471), so the basis of the map is production plus test and E9's
  figure is the one that does not match it.
- **Request**: amend **E9**'s body to 7.0k total / 4.8k production, and add that five of its twelve
  modules (`Schema`, `Health`, `Guard`, `Bin`, `(root)`) carry zero production findings, so the
  working set is seven modules and 4,837 lines - comparable to E2 and not the largest package in the
  programme. Without that sentence E9 reads as the package most likely to overrun.

### C4 - `BuiltinDirective/` is a directory, not a package

- **What the map says**: nothing.
- **What the tree says**: `crates/aphrodite/Source/BuiltinDirective/` holds 7 `.md` files and no
  Rust.
- **Evidence**: the filenames are directive identifiers (`focus`, `foresight`, `ccr-handling` and the
  rest) surfaced to the model and to `aphrodite_directive("add", ...)`; HOUSE-STYLE-REFACTOR's
  deviations section states that the directory is PascalCase while the data filenames are keys.
- **Request**: no card change. Recorded here so that a worker scanning `Source/` does not report an
  unowned module and does not "tidy" a directive identifier into a Rust name - renaming one of those
  files removes a directive from the model's vocabulary.

## 6. Completion criteria for the epic

Done means the following claims, each settled by a command, not by a summary. The baselines are the
numbers at HEAD `a98bd3b7`, so a claim is a comparison against a fixed figure rather than against
memory.

**1. No dummy scrutinee.** `grep -rn 'match ()' crates/aphrodite/Source crates/aphrodite-hermes/Source`
prints nothing. Baseline 20, all in `aphrodite` (0 in `aphrodite-hermes`).

**2. No artificial error ends a traversal.**
`grep -rnE -e 'Err\(\(\)\)' -e 'Ok::<\(\), \(\)>' crates/aphrodite/Source crates/aphrodite-hermes/Source`
prints nothing. Baseline 23 (20 in `aphrodite`, 3 in `aphrodite-hermes`).

**3. No facade re-export came back.**
`grep -rn 'pub use' crates/aphrodite/Source crates/aphrodite-hermes/Source` prints nothing. Baseline
0, so this claim is that it stays 0 - a phase-3 sweep already deleted all 189 of them.

**4. No boolean match with a no-op arm, and no single-arm match restating an `if`.**
`cargo clippy -p aphrodite -p aphrodite-hermes --all-targets -- -D warnings` exits 0 **with the two
allows removed** (C2). The grep
`grep -rnE '^\s*(true|false) => ' crates/aphrodite/Source crates/aphrodite-hermes/Source` reads 956 at
baseline; it is the size of the residue the lint has to consume. The grep is the coarse net - it
cannot tell an arm from a line in a doc comment - and clippy is the verdict, so a hit that survives the
lint is either inside a string or a comment, or it is a real arm that the card names and justifies.

**5. Every package cuts its `if` count by at least 60%.** The §7 probe, per package, against the §7
table. A package that lands above 40% of its baseline lists each surviving site and why the rule does
not reach it (a match guard inside a structural match, a `#[cfg]`, a const-context branch, or an
ordered rejection sequence that the style guide §3 explicitly keeps as guards).

**6. The ladder is green.** `cargo check -p aphrodite -p aphrodite-hermes --all-targets`,
`cargo test -p aphrodite`, `cargo test -p aphrodite-hermes`,
`cargo clippy -p aphrodite -p aphrodite-hermes --all-targets -- -D warnings` and
`cargo fmt --all -- --check` all exit 0, and `python3 Maintain/check_ffi_contract.py` prints `PASS`
with 0 violations.

**7. Nothing outside the epic's paths was edited.** `git status --short` names no path outside
`crates/aphrodite/Source`, `crates/aphrodite-hermes/Source` and `.hermes/`. The epic makes no git
writes, so the diff is the evidence and the auto-committer's commits are not.

**8. The defect register is closed, row by row.** D1 fixed (the guard scoped, the walk taking the map
by reference, the two leaves split); D2 recorded as refuted and the dead arm removed; D3 fixed (one
predicate decides streaming, from the parsed top-level field); D4 fixed (the bound owned by the type
that mutates); D5 fixed (the record boundary terminates the subject scan); D6 fixed (a poll's exit code
applies to the task it names); D7 fixed (the accumulator moved, not copied). Each row's handoff entry
names the change and the input that behaves differently - a green `cargo test` is not evidence for any
of them on its own, because the existing suite passes at HEAD.

**9. The consumer surfaces are accounted for.** Every `Consumer surface` row any package reported is
either empty or lists a name that did not change; no row may read "renamed, no consumer". The four
surfaces that make this claim checkable are the C ABI symbols
(`python3 Maintain/check_ffi_contract.py`), the metric names (a `/metrics` read), the `APHRODITE_*`
environment names and TOML keys (a configuration resolution), and the marker format (a compress then
retrieve round trip).

## 7. The `if` baseline

The probe, verbatim, so the number is reproducible and the verdict card measures against a fixed
figure rather than a remembered one:

```console
grep -rnE '(^|[^A-Za-z0-9_])if[ (]' <package paths> --include=*.rs | grep -vE '^\S+:[0-9]+:\s*//' | wc -l
```

Run per package, with `<package paths>` replaced by that package's owned prefixes. The probe scans
every `.rs` under the paths, including `Test/` files, so the files and lines columns below are the
full tree under the prefix, not the production subset §2 reports. The last two rows are the prefixes
no package owns (C1, C2) and are listed so the column reconciles.

| Package                    | `if` count |     Files |      Lines |
| -------------------------- | ---------: | --------: | ---------: |
| E1                         |          5 |       142 |      4,190 |
| E2                         |         20 |       172 |      6,226 |
| E3                         |         10 |       124 |      4,037 |
| E4                         |         12 |        91 |      2,803 |
| E5                         |         31 |       145 |      5,542 |
| E6                         |         20 |       169 |      6,048 |
| E7                         |          3 |       121 |      4,471 |
| E9                         |         16 |       250 |      7,022 |
| `Marker/` (unassigned, C1) |          4 |        17 |        964 |
| `(root)` (unassigned, C2)  |          0 |         2 |        119 |
| **Total**                  |    **121** | **1,233** | **41,422** |

The total reconciles with the whole-crate probes: 105 in `crates/aphrodite/Source` and 16 in
`crates/aphrodite-hermes/Source`, 121 together, over 1,233 `.rs` files - the same file count the audit
reports for the two crates.

Two readings of the table matter for the verdict. First, the distribution is not proportional to size:
E5 carries 31 of the 121 on 13% of the lines, because `Preview` and `StructExtract` are where the
ordered-rejection and budget shapes concentrate, while E7 carries 3 on 11% of the lines. A single
global percentage hides that, which is why the target is per package. Second, a count is not the
goal - the style guide §2 says so in as many words: a `match ()` with four guards has four branches
and less clarity than the `if` chain it replaced. The 60% figure is a floor that makes the rewrite
measurable, not a definition of it.
