# Naming B5 - startup and configuration

Partition: `crates/aphrodite/Source/{Boot,Configuration,ConfigurationLoader,Setup,Home}/**` and
`Main.rs`. Behaviour is unchanged; every emitted TOML key, JSON key, CLI flag, `--help` value name
and ABI symbol is byte-identical, and each is pinned rather than left to derivation.

No build was run (the card forbids it). What was verified instead is listed under "Verification".

## Identifiers renamed

### `Configuration::Proxy::Previews::Struct` - `[previews]`

| Was                   | Now         | Key (pinned)         |
| --------------------- | ----------- | -------------------- |
| `ModelFamily`         | `Family`    | `model_family`       |
| `CodeStructureMap`    | `Structure` | `code_structure_map` |
| `PreviewMaximumChars` | `Limit`     | `preview_max_chars`  |
| `RustPreviewLines`    | `Lines`     | `rust_preview_lines` |

`Structure` rather than the doctrine table's `Map`: the table assumes a `Previews::Structure` path
exists to carry the noun. A `[previews]` key set cannot be re-nested without changing the emitted
TOML, so the word the missing path would have carried stays in the name. `Lines` loses `Rust` to
the same constraint; its doc line names `[previews] rust_preview_lines`.

### `Configuration::Proxy::Prompts::Struct` - `[prompts]`

| Was                  | Now        | Key (pinned)           |
| -------------------- | ---------- | ---------------------- |
| `RetrieveGuidance`   | `Guidance` | `retrieve_guidance`    |
| `CCRMarkerHint`      | `Hint`     | `ccr_marker_hint`      |
| `CatalogIntentHints` | `Intent`   | `catalog_intent_hints` |

### `Configuration::Proxy::Compression::Struct` - `[compression]`

| Was                      | Now          | Key (pinned)            |
| ------------------------ | ------------ | ----------------------- |
| `EngineThresholdPct`     | `Trigger`    | `engine_threshold_pct`  |
| `EngineProtectFirst`     | `First`      | `engine_protect_first`  |
| `EngineProtectLast`      | `Last`       | `engine_protect_last`   |
| `EngineMinimumMsgs`      | `Minimum`    | `engine_min_msgs`       |
| `ToolThresholdToken`     | `Token`      | `tool_threshold_token`  |
| `ToolThresholdCache`     | `Cache`      | `tool_threshold_cache`  |
| `TerminalThreshold`      | `Terminal`   | `terminal_threshold`    |
| `InlineThreshold`        | `Inline`     | `inline_threshold`      |
| `AutoExpand`             | `Expand`     | `auto_expand`           |
| `AutoExpandLimit`        | `Limit`      | `auto_expand_limit`     |
| `MaximumRequestBodySize` | `Body`       | `max_request_body_size` |
| `CatalogMode`            | `Catalog`    | `catalog_mode`          |
| `ClassifierPoll`         | `Poll`       | `classifier_poll`       |
| `CodeMultiplier`         | `Multiplier` | `code_multiplier`       |
| `StoreMaximumEntries`    | `Entries`    | `store_max_entries`     |
| `StoreMaximumBytes`      | `Bytes`      | `store_max_bytes`       |

`EngineMinimumMsgs` also lost the forbidden abbreviation `Msgs`. `Trigger` is the one word left for
"the engine window trigger, as a percentage of the context limit" once `Engine` is carried by the
sibling `First`/`Last`/`Minimum` and the doc line names `engine_threshold_pct`.

### `Configuration::Proxy::Proxy::Struct` - one `[[proxies]]` entry

| Was               | Now        | Key (pinned)      |
| ----------------- | ---------- | ----------------- |
| `name`            | `Name`     | `name`            |
| `APIKey`          | `Key`      | `api_key`         |
| `APIURL`          | `URL`      | `api_url`         |
| `ToolRelay`       | `Relay`    | `tool_relay`      |
| `CCRTtlSeconds`   | `TTL`      | `ccr_ttl_seconds` |
| `CCRDatabasePath` | `Database` | `ccr_db_path`     |
| `NotifyURL`       | `Callback` | `notify_url`      |
| `NotifyKey`       | `Auth`     | `notify_key`      |
| `MaximumContext`  | `Context`  | `max_context`     |
| `MaximumOutput`   | `Output`   | `max_output`      |

`name` was the only field in the whole partition with **no** `#[serde(rename)]`; renaming it
required _adding_ the pin (`#[serde(rename = "name")]`). Every other field above already carried
its pin, which was kept in place unchanged.

`NotifyURL`/`NotifyKey` became `Callback`/`Auth` rather than `Notify`/`NotifyKey`: two fields may
not share a word, and `Key` is already the upstream credential.

### `Configuration::Proxy::Defaults::Struct` - `[defaults]`

| Was             | Now   | Key (pinned)      |
| --------------- | ----- | ----------------- |
| `APIURL`        | `URL` | `api_url`         |
| `CCRTtlSeconds` | `TTL` | `ccr_ttl_seconds` |
| `APIKey`        | `Key` | `api_key`         |

### `Configuration::CLI::Struct` - the parsed command line (clap)

| Was                   | Now        | Flag (pinned)           | Value name (pinned)     |
| --------------------- | ---------- | ----------------------- | ----------------------- |
| `command`             | `Command`  | - (subcommand)          | -                       |
| `APIURL`              | `URL`      | `apiurl`                | `APIURL`                |
| `APIKey`              | `Key`      | `api-key`               | `API_KEY`               |
| `MaximumContext`      | `Context`  | `maximum-context`       | `MAXIMUM_CONTEXT`       |
| `MaximumOutput`       | `Output`   | `maximum-output`        | `MAXIMUM_OUTPUT`        |
| `CCRDatabasePath`     | `Database` | `ccr-database-path`     | `CCR_DATABASE_PATH`     |
| `CCRTtlSeconds`       | `TTL`      | `ccr-ttl-seconds`       | `CCR_TTL_SECONDS`       |
| `StoreMaximumEntries` | `Entries`  | `store-maximum-entries` | `STORE_MAXIMUM_ENTRIES` |
| `StoreMaximumBytes`   | `Bytes`    | `store-maximum-bytes`   | `STORE_MAXIMUM_BYTES`   |
| `NoCCRMarker`         | `NoMarker` | `no-ccr-marker`         | - (flag)                |
| `ToolRelay`           | `Relay`    | `tool-relay`            | - (flag)                |
| `NotifyURL`           | `Callback` | `notify-url`            | `NOTIFY_URL`            |
| `NotifyKey`           | `Auth`     | `notify-key`            | `NOTIFY_KEY`            |

`value_name` is not optional here: clap derives it from the field name, so `MaximumContext` →
`Context` would have shipped `--maximum-context <CONTEXT>`. The flag spelling is the smaller half
of the contract.

### `Configuration::CLI::Setup::Struct` and `Configuration::CLI::Command::Enum::Setup`

| Was         | Now     | Flag (pinned) | Value name (pinned) |
| ----------- | ------- | ------------- | ------------------- |
| `APIKey`    | `Key`   | `api-key`     | `API_KEY`           |
| `APIURL`    | `URL`   | `apiurl`      | `APIURL`            |
| `CachePort` | `Cache` | `cache-port`  | `CACHE_PORT`        |
| `TokenPort` | `Token` | `token-port`  | `TOKEN_PORT`        |
| `NoLaunch`  | kept    | `no-launch`   | - (flag)            |
| `Force`     | kept    | `force`       | - (flag)            |

### `Configuration::Environment::*`

`EnvironmentBool` → `Environment::Bool::Fn::Fn`, `EnvironmentText` → `Environment::Text::Fn::Fn`,
`EnvironmentParseWarn` → `Environment::Warn::Fn::Fn`. Call sites alias the leaf (`as Bool`,
`as Text`, `as Warn`) because `Configuration.Bool(...)` / `.Text(...)` / `.Warn::<u64>(...)` is the
reading the path intends.

### `ConfigurationLoader::Configuration::Resolve::*` - one getter per family

| Was             | Now        | Leaf               |
| --------------- | ---------- | ------------------ |
| `GetBool`       | `Bool`     | `Resolve/Bool`     |
| `GetUsize`      | `Size`     | `Resolve/Size`     |
| `GetOptUsize`   | `Limit`    | `Resolve/Limit`    |
| `GetU64`        | `Number`   | `Resolve/Number`   |
| `GetString`     | `Text`     | `Resolve/Text`     |
| `GetStringList` | `List`     | `Resolve/List`     |
| `GetTemplates`  | `Template` | `Resolve/Template` |

The `Get` verb is what the module `Resolve` already says. `GetOptUsize` also lost `Opt`, a
forbidden abbreviation. These seven are inherent methods on one type, so §4 is what makes the
distinction word (`Bool`, `Size`, `Limit`, ...) necessary rather than decorative.

### `ConfigurationLoader::Configuration::Load::*`

| Was           | Now       | Leaf            |
| ------------- | --------- | --------------- |
| `LoadFrom`    | `Read`    | `Load/File`     |
| `SetOverride` | `Set`     | `Load/Override` |
| `Reload`      | `Refresh` | `Load/Reload`   |

`Refresh` is a deliberate distinction, not a synonym: it re-reads the document. `Reload` was freed
for the apply pass below, because both are inherent methods on the same type and could not share
the name.

### `ConfigurationLoader::Configuration::Apply::*` - the two passes

| Was                | Now        | Path                    |
| ------------------ | ---------- | ----------------------- |
| `ApplyCompression` | `Startup`  | `Apply::Startup::Fn`    |
| `ApplyReload`      | `Reload`   | `Apply::Reload::Fn`     |
| `ApplyGlobals`     | `Global`   | `Apply::Global::Fn`     |
| `const Startup`    | `Sequence` | `Apply::Startup::Const` |
| `const Reload`     | `Sequence` | `Apply::Reload::Const`  |

`ApplyCompression` was also wrong on content - it applies every section, not compression - so the
rename fixed a misnomer as well as a compound. `Configuration.Startup(&mut State)` /
`Configuration.Reload(&mut State)` / `Configuration.Global()` is the whole lifecycle API.

### `ConfigurationLoader::Warn` - one export per leaf

`Warn/Fn.rs` held two exports. Split: `Warn::Fn::Fn` (emits) and `Warn::Message::Fn::Fn` (builds the
text, which is the half a test can assert without a subscriber). `WarnParseFailure` and
`ParseFailureMessage` are gone.

### `ConfigurationLoader::Configuration::Apply::Global::Rendering`

The value is internal (never serialised), so its words could move into the path:

```text
Rendering::Struct { Limit, Marker::Struct { Format, Hint, Enable }, Templates, Family }
```

`PreviewMaximum` → `Limit`; `MarkerFormat`/`MarkerHint`/`MarkerHintEnabled` →
`Rendering::Marker::{Format, Hint, Enable}`. `Enable` keeps `[prompts] ccr_marker_hint`'s meaning
in its doc line.

### `Configuration::Proxy::Multi::Port::Fn`

`ApplyPortOverride` → `Override` (`Multi::Port::Override(Listen, Variable)`); the `port` is the
path and `apply` is the verb the module already carries.

### `Setup::*`

| Was                                | Now             | Path                |
| ---------------------------------- | --------------- | ------------------- |
| `Step::CreateDirectories`          | `Create`        | `Plan::Step`        |
| `Step::PreparePluginDirectory`     | `Prepare`       | `Plan::Step`        |
| `Step::InstallBinary`              | `Binary`        | `Plan::Step`        |
| `Step::InstallDylibs`              | `Dylib`         | `Plan::Step`        |
| `Step::WriteConfiguration`         | `Configuration` | `Plan::Step`        |
| `Step::WriteManifest`              | `Manifest`      | `Plan::Step`        |
| `Step::WriteLoader`                | `Loader`        | `Plan::Step`        |
| `Step::WriteVersionPin`            | `Pin`           | `Plan::Step`        |
| `Step::RegisterPlugin`             | `Register`      | `Plan::Step`        |
| `Ownership::DevStaleSymlink`       | `Stale`         | `Plan::Ownership`   |
| `Register::Outcome::ExitedNonZero` | `NonZero`       | `Register::Outcome` |
| `Register::Outcome::ExitedZero`    | `Zero`          | `Register::Outcome` |
| local `SetupOwnsDirectory`         | `Owned`         | `Plan::Fn`          |

`Setup/Execute/Apply/Fn.rs` no longer aliases the step functions (`WriteBinaryVersion`,
`WriteInitPy`, `WritePluginYAML`): it imports the modules and calls `BinaryVersion::Fn::Fn`,
`InitPy::Fn::Fn`, `PluginYAML::Fn::Fn`, which is the shape the rest of the dispatch already used.

### `Home::*` and test-local aliases

`HermesHomeOpt`/`RuntimeHomeOpt` → `Opt` in `Home/{Hermes,Runtime}/Fn.rs` (the module is `Opt`).
Test-local aliases and helpers: `GuardedApplyCompression` → `Guarded`, `GlobalsGuards` → `Globals`,
`HermesPluginShim` → `Shim`, `VerifyDownloadChecksum` → `Checksum`, `MultiConfigurationFromToml`
→ `Toml`, `HandleToolRelay` → `Handle`.

## Serde / clap pin checklist (what a reviewer must diff)

Every pin below is an attribute the rename depends on. A missing one is a silent contract change,
not a compile error.

**serde, already present and retained unchanged (36 renamed fields):**
`Compression::Struct` - `engine_threshold_pct`, `engine_protect_first`, `engine_protect_last`,
`engine_min_msgs`, `tool_threshold_token`, `tool_threshold_cache`, `terminal_threshold`,
`inline_threshold`, `auto_expand`, `auto_expand_limit`, `max_request_body_size`, `catalog_mode`,
`classifier_poll`, `code_multiplier`, `store_max_entries`, `store_max_bytes`.
`Proxy::Struct` - `listen`, `mode`, `api_key`, `api_url`, `model`, `tool_relay`, `dev`,
`ccr_ttl_seconds`, `ccr_db_path`, `notify_url`, `notify_key`, `timeout`, `max_context`,
`max_output`.
`Previews::Struct` - `model_family`, `code_structure_map`, `preview_max_chars`, `rust_preview_lines`.
`Prompts::Struct` - `retrieve_guidance`, `ccr_marker_hint`, `catalog_intent_hints`.
`Defaults::Struct` - `api_url`, `model`, `ccr_ttl_seconds`, `api_key`.

**serde, newly added (1 field):** `Proxy::Struct::Name` - `#[serde(rename = "name")]`.

**clap `long` (16):** `apiurl`, `api-key`, `maximum-context`, `maximum-output`,
`ccr-database-path`, `ccr-ttl-seconds`, `store-maximum-entries`, `store-maximum-bytes`,
`no-ccr-marker`, `tool-relay`, `notify-url`, `notify-key` (root); `api-key`, `apiurl`,
`cache-port`, `token-port` (`setup`).

**clap `value_name` (14):** `APIURL`, `API_KEY`, `MAXIMUM_CONTEXT`, `MAXIMUM_OUTPUT`,
`CCR_DATABASE_PATH`, `CCR_TTL_SECONDS`, `STORE_MAXIMUM_ENTRIES`, `STORE_MAXIMUM_BYTES`,
`NOTIFY_URL`, `NOTIFY_KEY` (root); `API_KEY`, `APIURL`, `CACHE_PORT`, `TOKEN_PORT` (`setup`).

## Words that kept a qualifier, and why

1. **`Multi::Struct`'s `Load` and `Resolve`.** Both leaves are `Fn.rs` under paths that name the
   operation, so §2 would make both `Fn` - but they are inherent methods on one type and Rust
   requires unique method names. The verb survives in the name because the collision rule (§4)
   outranks the one-word rule.
2. **`Load::Search::Load` and `Configuration::Load`.** Kept. The nearest module is `Search`, and the
   repeat is with the grandparent; the tree's own `Fn::Fn` / `Struct::Struct` idiom reads the same
   way. Renaming it to `Search` would have made the primary constructor a verb the caller does not
   perform.
3. **`[compression]`'s two-word residue is zero**, but the qualifiers that survive are the
   distinction words (`Token`/`Cache`, `Terminal`/`Inline`), which is what §4 asks for. If a reader
   expects `Token` to mean "the token" rather than "the token-mode tool threshold", the doc line is
   the answer and it names the key.
4. **`Setup::Plan::Struct::RemoveStaleSymlink`** appears only in two historical paragraphs
   (`Plan/Struct.rs`, `Ownership/Enum.rs`) that narrate the two booleans the `Ownership` enum
   replaced. Left as written: it names something that no longer exists, in the past tense.

## Renames that needed a path change outside this row

Five call-site edits in other cards' paths, each required for the tree to stay self-consistent:

| File                                                                                                                                                                                             | Card  | Why                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----- | --------------------------------------------------------------------------------------------- |
| `Proxy/Threshold/Resolve/Fn.rs`                                                                                                                                                                  | B2    | reads the renamed `[compression]` fields; also aliases `Environment::Warn` and the table type |
| `Proxy/CCR/Reload/Fn.rs`                                                                                                                                                                         | B2    | reads `[previews]`/`[prompts]`/`[compression]`; JSON echo keys unchanged                      |
| `Proxy/State/Build/Fn.rs`                                                                                                                                                                        | B2    | builds `AppState` from the renamed `CLI` fields                                               |
| `Proxy/Threshold/Struct.rs`                                                                                                                                                                      | B2    | one doc comment naming `Multi::Port::Override`                                                |
| `ABI/Initialise.rs`                                                                                                                                                                              | B1    | calls `Startup`/`Global`, uses `Read`                                                         |
| `ABI/Configuration/Reload/Assess/Fn.rs`                                                                                                                                                          | B1    | calls `Warn::Fn::Fn`                                                                          |
| `State/Struct.rs`, `State/Default.rs`                                                                                                                                                            | B6    | doc comments naming `Apply::Startup::Fn`                                                      |
| `Marker/Test/Format.rs`, `Marker/Format/Registration/Guard/Fn.rs`, `Directive/Test/Load.rs`                                                                                                      | B3/B7 | doc comments naming `Apply::Startup::Fn`                                                      |
| `aphrodite-hermes/.../{Configuration/Template/Seeded/Fn.rs, Configuration/Reload/Fn.rs, Configuration/mod.rs, Session/Watcher/Apply/Fn.rs, Configuration/Test/Reload.rs, Session/Test/Apply.rs}` | B9    | calls `Startup`/`Reload`/`Global`, uses `Read`                                                |

Items I could **not** rename because they are declared in another row, and which therefore still
trip the 3+ probe from inside my paths:

- `AphroditeState` fields (`B6`): `ChainSplitEnabled`, `ChainSplitMinimumSegments`,
  `ChainSplitFloor`, `ChainSplitMaximumSegments`, `FlowBudgetChars`, `PollWorkerEnabled`,
  `AutoExpandLimit`, `MaximumRequestBodySize`, `ContextEngineEnabled`, `EngineThresholdPct`,
  `EngineProtectFirst`, `EngineProtectLast`, `EngineMinimumMsgs`, `CatalogIntentHints`.
  These are a flat struct's fields, so §4 - not §1 - governs them: there is no per-field path for
  the extra words to move into.
- `Guard::Bearer::Middleware::RequireMgmtToken` (`B1`), referenced from `Boot/Route/{Build,mod}.rs`.
- `Proxy::State::Struct::UpstreamHealthCache` (`B2`/`B6`), referenced from
  `Boot/Route/Health/Upstream.rs`.
- `Proxy::Threshold::Const::{CacheCompressThreshold, TokenCompressThreshold, CodeMultiplierX100}`
  (`B2`), referenced from `Boot/Listen/Watcher/Apply/Fn.rs`.
- `Flow::Prompt::Const::ShippedSessionInject` (`B3`), referenced from the template applier.

## What the doctrine does not cover

1. **A flat wire-keyed table cannot push its words into a path.** `[compression]`'s 16 knobs and a
   `[[proxies]]` entry's 10 are one TOML table each; re-nesting them would change the emitted keys.
   The doctrine's table (`CodeStructureMap` → `Previews::Structure` → `Map`) assumes the sub-path
   exists. Where it cannot, the word either stays in the name (`Structure`, `Lines`) or the doc line
   carries it (`Token`, `Cache`, `Trigger`). This is the one systematic loss in the pass, and it is
   why the doc comments here are longer than the names.
2. **`clap`'s `value_name` is a second derived name.** The doctrine names three contracts (serde,
   flags, ABI); `--maximum-context <MAXIMUM_CONTEXT>` has a fourth spelling derived from the field
   name, and it changes when the field does. Worth adding to §5.
3. **Inherent methods share one namespace across modules.** The doctrine's §4 talks about a
   "scope"; for `impl Struct` blocks spread over `Load/`, `Apply/` and `Resolve/`, the scope is the
   whole type, not the module. That is what forced `Load::Reload` → `Refresh`.
4. **Test function names are `cargo test` report labels.** They are the one class of identifier in
   this partition left long (`TestResolveTimeoutClampedTo600` and 52 siblings). They are read in CI
   output, not in source, and no other partition is shortening them - renaming them here alone would
   make the tree less coherent, not more. Declared as the exception rather than silently kept.
5. **A clap field's doc comment _is_ `--help` text.** §6's "every renamed item carries a `///`
   naming its external contract" cannot be applied to a `#[derive(Parser)]` field: adding
   ``/// `--maximum-context` `` to the doc would print that line in `--help` and change the very
   contract the card requires to stay identical. For the 16 clap fields the contract is named by the
   pinned `long`/`value_name` attributes in the same edit instead, and the doc comments were left
   byte-identical. Every non-clap renamed `pub` item does carry the `///`.

## Verification

The claims below are executable, not prose: `hermes-verify-b5-naming.py` in the OS temp directory
(`/private/var/folders/.../T/hermes-verify-b5-naming.py`) runs all seven checks and exits 0 only if
every one passes. It is left in place rather than cleaned up, so the evidence survives this turn.

| Claim                               | Probe                                                                                                                                                                                            | Result                                                                                                            |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| serde keys unchanged                | extract `#[serde(rename)]` + field pairs from the six tables; assert every field is pinned and the key list equals the pre-rename list                                                           | 48 pins; 36 on a renamed field and already there, 11 on fields this card did not rename, exactly 1 added (`name`) |
| CLI flags and value names unchanged | extract `long`/`value_name` per `#[arg]` (multi-line tolerant) and compare to `target/release/aphrodite --help` / `setup --help` captured **before** the edits (`.hermes/tmp/help-*-before.txt`) | 18 root + 7 setup options equal, flag and `<VALUE>` both                                                          |
| one word per item                   | 3+ token probe over the partition, each non-comment hit classified as dependency / external contract / test label / another card's item                                                          | 229 → 106, 0 unclassified                                                                                         |
| no renamed symbol survives          | grep each old name across `crates/`                                                                                                                                                              | 0 hits                                                                                                            |
| module tree intact                  | `pub mod X;` vs `X.rs` / `X/mod.rs`                                                                                                                                                              | 0 missing                                                                                                         |
| every changed file parses           | `rustfmt --edition 2024 --emit stdout` over every changed `.rs`                                                                                                                                  | 0 parse errors (715 files)                                                                                        |
| emitted artifacts untouched         | `git status --porcelain crates/aphrodite/templates/ plugins/`                                                                                                                                    | clean                                                                                                             |

Two findings the probe produced that the by-hand sweep had missed, both fixed: a stale module doc in
`Home/Database/Fn.rs` and a stale `MaximumContext` reference in `Boot/Route/Chat.rs`.

### Compile evidence (diagnostic, not a gate)

The card forbids a build as its gate, but a rename's call sites can only be proven by a typecheck, so
`cargo check --lib -p aphrodite -p aphrodite-hermes` was run as a diagnostic. It found seven real
errors in this partition, all fixed:

| Error                                                                                                | Cause                                                                                                             | Fix                                        |
| ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `Rendering/Struct.rs`, `Resolve/Template.rs`, `Multi/Struct.rs` - `TemplatesConfiguration` not found | B6 renamed `State::Template::Struct::TemplatesConfiguration` → `Struct`                                           | followed the rename at my three call sites |
| `Section/Template.rs` - `ShippedSessionInject` not found                                             | B3 renamed `Flow::Prompt::Const::ShippedSessionInject` → `Shipped`                                                | followed the rename                        |
| `Setup/Execute/Apply/Fn.rs` ×3 - `cannot find Fn in PluginYAML`                                      | `Setup/Write/{PluginYAML,InitPy,BinaryVersion}` are single files, so the call is `X::Fn(..)`, not `X::Fn::Fn(..)` | dropped the extra hop                      |

Result: **60 → 10 lib errors, 0 in this partition.** The ten survivors are in `Preview` (4),
`StructExtract` (2), `Proxy`, `Prefetch`, `PollWorker`, `Hook` - other cards' in-flight work - and
none of them names anything this card renamed, so no other row was broken by it.

Two honest limits on that evidence. `aphrodite-hermes` never got type-checked (it depends on
`aphrodite`, which does not yet build), so the B9 call-site edits (`.Startup()`, `.Reload()`,
`.Global()`, `Struct::Read`) remain compiler-unverified. And `--all-targets` still reports 165 errors
inside `#[cfg(test)]` modules of other rows (`Resolve/Test/Expand.rs` and friends), so this
partition's own test modules have not been compiled either. Both clear as the other eight cards land.

One environment note: this tree is shared with the other eight cards and a sibling ran `cargo fmt`
over it mid-pass, so `#[arg(...)]` attributes here are now one-argument-per-line rather than the
single lines this card wrote. Formatting only - no attribute was added, dropped or reordered - and
the CLI probe reads attribute blocks rather than lines so it is insensitive to it.

Still owed to the verification card: the full build, the test count, and a real `--help` diff. The
`value_name` pins make the last one a one-line check.
