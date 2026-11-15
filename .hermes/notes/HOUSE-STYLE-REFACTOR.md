# House-Style Refactor - Aphrodite crates

Both crates under `crates/` adopt the PlayForm Rust house style
(`PlayForm/Cargo/Run`, `Dotology`): one export per file, item named by kind,
deeply nested PascalCase modules grouped by function, `mod.rs` facades.

Package identity is unchanged: `crates/aphrodite` stays `aphrodite`,
`crates/aphrodite-hermes` stays `aphrodite-hermes`, and every
`libaphrodite_hermes.dylib` path, CI alias, release asset and plugin loader
reference stays valid.

## The contract

1. **Crate root is `Source/Library.rs`** (lib) and `Source/Main.rs` (bin).
   `Cargo.toml` carries `[lib] path = "Source/Library.rs"` and
   `[[bin]] path = "Source/Main.rs"`. `Library.rs` holds crate docs, the
   `#![allow(non_snake_case)]` gate, module declarations and re-exports -
   nothing else.
2. **A module is a PascalCase directory** with a `mod.rs` facade listing
   `pub mod <Leaf>;` one per line.
3. **One export per file.** A leaf file owns exactly one item and names it by
   kind: `Struct`, `Enum`, `Fn`, `Trait`, `Const`. The directory path carries
   the domain, the item name carries the kind. Consumers path to it:
   `Preview::Builder::Build::Fn`, `State::Marker::Struct`.
4. **Full words, never abbreviations.** `Configuration` not `Config`,
   `Environment` not `Env`, `Language` not `Lang`, `Number` not `Num`,
   `Directory` not `Dir`.
5. **Acronyms and product names fully capitalised**: `URL`, `URI`, `ID`,
   `JSON`, `XML`, `HTML`, `CSV`, `YAML`, `JS`, `CLI`, `API`, `SDK`, `CCR`,
   `SSE`, `ABI`, `FFI`, `LLM`, `MCP`, `RPC`, `gRPC` as `GRPC`.
   In an identifier, a path component or a filename, never the lowercase-Camel
   tail: `Abi`, `Api`, `Ccr`, `Cli`, `Sse`, `Ffi`, `Llm`, `Mcp`, `Rpc`,
   `Grpc`, `Id`, `Url`, `Uri`, `Json`, `Xml`, `Html`, `Csv`, `Yaml`, `Js`,
   `Macos`. `snake_case` identifiers keep the acronym lowercase (`ccr_marker`,
   `api_url`) - there `ccr` is already fully capitalised, and `fn CCRMarker`
   would fail `non_snake_case` under `-D warnings`. A product name keeps
   its published spelling where the identifier allows it and its capitalised
   form where it does not: the Apple platform is `MacOS` (never `Macos`),
   the Apple product in prose is `macOS`.
6. **Singular, action-oriented, present tense.** `Action` not `Actions`,
   `Hook` not `Hooks`, `Template` not `Templates`. A directory that groups
   behaviour is named for the shared verb or purpose: `Render/`, `Parse/`,
   `Resolve/`, `Load/`, `Store/`.
7. **Deep nesting is the default.** A leaf that is a group of related items
   becomes another directory. A leaf file over ~150 lines is a signal to nest
   once more; the practical ceiling is one screen.
8. **Facades re-export, they do not define.** A facade may `pub use` a
   child's item when the child owns the module's canonical item
   (`Token/mod.rs` → `pub use Token::Struct;`), and must not carry logic.
9. **Documentation is part of the file.** Every module file opens with a
   `//!` block saying what the module is for and why it exists; every public
   item carries `///` docs in the house voice - what it does, what it is not,
   and the consequence of getting it wrong. No `//! TODO`.
10. **Tests stay with their module**: `<Module>/Test.rs`, declared
    `#[cfg(test)] mod Test;`. A test file over ~200 lines splits by area into
    `<Module>/Test/<Area>.rs`.
11. **Formatting**: rustfmt with the repo's `rustfmt.toml` (hard tabs,
    `space_after_colon = false`, `max_width = 120`).

### Deviations, and why

- **C ABI leaves keep their exported identifier.** `Source/ABI/Compress.rs`
  defines `pub extern "C" fn aphrodite_compress`, not `fn Fn`. The symbol
  name IS the ABI contract that `aphrodite-hermes`, the ctypes bindings and
  `Maintain/check_ffi_contract.py` assert; renaming it would break codegen
  (cbindgen → `_bindings.py`) and the plugin. One export per file still holds.
- **Struct field names stay `snake_case`.** Several structs are `serde`-
  serialised onto the proxy wire (`ToolRelayRequest`, `CCRNotification`, the
  configuration tables). Renaming fields changes JSON a third-party client
  sees - a wire break, not a style change.
- **`BuiltinDirective/` keeps lowercase `.md` filenames.** The filenames are
  the directive identifiers (`focus`, `foresight`, `ccr-handling`) surfaced to
  the model and to `aphrodite_directive("add", ...)`; the directory is
  PascalCase, the data filenames are keys.

## Naming applied in phase 1 (done)

`src/` → `Source/`, `lib.rs` → `Library.rs`, `main.rs` → `Main.rs`,
`tests.rs` → `Test.rs`, `builtin_directives/` → `BuiltinDirective/`.

| Old                    | New                            | Old                  | New                   |
| ---------------------- | ------------------------------ | -------------------- | --------------------- |
| `catalog.rs`           | `Catalog/`                     | `hooks.rs`           | `Hook/`               |
| `chain_split.rs`       | `ChainSplit/`                  | `marker/`            | `Marker/`             |
| `config_loader.rs`     | `ConfigurationLoader/`         | `poll_worker.rs`     | `PollWorker/`         |
| `config/`              | `Configuration/`               | `prefetch.rs`        | `Prefetch/`           |
| `config/cli.rs`        | `Configuration/CLI.rs`         | `preview/builders/`  | `Preview/Builder/`    |
| `config/env.rs`        | `Configuration/Environment.rs` | `preview/detectors/` | `Preview/Detector/`   |
| `debug.rs`             | `Debug/`                       | `preview/line/`      | `Preview/Line/`       |
| `directives/`          | `Directive/`                   | `preview/text/`      | `Preview/Text/`       |
| `flow.rs`              | `Flow/`                        | `preview/lang.rs`    | `Preview/Language.rs` |
| `home.rs`              | `Home/`                        | `preview/type.rs`    | `Preview/Type.rs`     |
| `proxy.rs`             | `Proxy/`                       | `resolve/`           | `Resolve/`            |
| `retrieve.rs`          | `Retrieve/`                    | `setup/`             | `Setup/`              |
| `scripting.rs`         | `Scripting/`                   | `stage2.rs`          | `Stage2/`             |
| `session.rs`           | `Session/`                     | `state/`             | `State/`              |
| `struct_extract/`      | `StructExtract/`               | `templates.rs`       | `Template/`           |
| `hermes/schemas.rs`    | `Schema/`                      | `hermes/tools.rs`    | `Tool/`               |
| `hermes/directives.rs` | `Directive/`                   | `hermes/bin/`        | `Bin/`                |

Acronym leaves: `JSON`, `XML`, `HTML`, `CSV`, `YAML`, `JS`, `YAMLKey`,
`EnvelopeJSON`, `MarkdownHeading`, `MarkdownStructure`, `NumberBefore`.

## Module map - the target trees (phase 2)

### aphrodite

| Module                 | Leaves                                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Library.rs`           | crate docs, `pub mod`, `pub use Preview::{build_preview, detect_type}`                                             |
| `ABI/`                 | one file per `aphrodite_*` export; `Handle/{Table,Allocate,Lookup}.rs`; `Guard.rs`                                 |
| `Main.rs`              | `fn main()` only                                                                                                   |
| `Boot/`                | `Router/`, `Listen/`, `Shutdown/`, `Configuration/`                                                                |
| `Guard/`               | `Loopback/`, `Host/`, `Bearer/`, `Token/`                                                                          |
| `Proxy/`               | `State/`, `Threshold/`, `Body/`, `Cache/`, `CCR/`, `Relay/`, `Metadata/`, `Marker/`, `Test/`                       |
| `Hook/`                | `Live/`, `ToolResult/`, `Terminal/`, `Session/`, `LLM/`, `Path/`, `Test/`                                          |
| `ConfigurationLoader/` | `Config/`, `Warn/`, `Test/`                                                                                        |
| `PollWorker/`          | `Task/`, `Should/`, `Check/`, `Update/`, `Insert/`, `Status/`, `Test/`                                             |
| `Flow/`                | `Prompt/`, `Nudge/`, `Signature/`, `Window/`, `TurnContext/`, `Test/`                                              |
| `Retrieve/`            | `Live/`, `Page/`, `Filter/`, `Request/`, `Response/`, `Test/`                                                      |
| `Session/`             | `Start/`, `Turn/`, `Archive/`, `Index/`, `Catalog/`, `Engine/`, `Test/`                                            |
| `Prefetch/`            | `Arrival/`, `Read/`, `Outcome/`, `Insert/`, `Status/`, `Test/`                                                     |
| `Template/`            | `Vars/`, `Extract/`, `Render/`, `Set/`, `Get/`, `Family/`, `Test/`                                                 |
| `Stage2/`              | one reducer per file, `Registry/`, `Compress/`, `Test/`                                                            |
| `Debug/`               | `Level/`, `Log/`, `Counter/`, `Timing/`, `Metrics/`, `Path/`, `Test/`                                              |
| `Catalog/`             | `Build/`, `Format/`, `Size/`, `Test/`                                                                              |
| `Scripting/`           | `Engine/`, `Script/`, `Payload/`, `Test/`                                                                          |
| `ChainSplit/`          | `Segment/`, `Split/`, `MarkedCommand/`, `MarkedOutput/`, `ErrorHint/`, `Test/`                                     |
| `Home/`                | `Hermes/`, `Runtime/`, `Plugin/`, `Environment/`, `Tilde/`, `Path/`, `Test/`                                       |
| `State/`               | `Struct/`, `Default/`, `Const/`, `Template/` + `ChainSplit/`, `Directive/`, `Event/`, `File/`, `Marker/`, `Store/` |
| `Directive/`           | `Action/`, `Build/`, `Builtin/`, `Load/`, `Model/`, `Test/`                                                        |
| `Marker/`              | `Format/`, `Parse/`, `Test/`                                                                                       |
| `Preview/`             | `Builder/`, `Detector/`, `Line/`, `Text/`, `Input/`, `Language/`, `Type/`, `Detect/`, `State/`, `Test/`            |
| `Resolve/`             | `Expand/`, `Filter/`, `One/`, `Parse/`, `Recursive/`, `Test/`                                                      |
| `Setup/`               | `Dylib/`, `Write/`, `Run/`, `Verify/`, `MacOS/`, `Test/`                                                           |
| `StructExtract/`       | `Detect/`, `Format/`, `Go/`, `JS/`, `Python/`, `Rust/`, `Test/`                                                    |
| `Configuration/`       | `CLI/`, `Environment/`, `Proxy/`, `Test/`                                                                          |

### aphrodite-hermes

| Module       | Leaves                                                             |
| ------------ | ------------------------------------------------------------------ |
| `Library.rs` | crate docs, `pub mod`, re-exports                                  |
| `Session/`   | `Registry/`, `Lock/`, `With/`, `Broadcast/`, `Watcher/`            |
| `ABI/`       | one file per `aphrodite_hermes_*` export                           |
| `Hook/`      | `FailOpen/`, `Replacement/`, `PostTool/`, `Log/`                   |
| `Tool/`      | `Registry/`, `Dispatch/`, `Read/`, `Unwrap/`, `Compress/`, `Test/` |
| `Schema/`    | one file per tool schema, `All/`, `Get/`, `Test/`                  |
| `Directive/` | `Resolve/`, `Builtin/`, `Materialize/`, `Test/`                    |
| `Debug/`     | `Flag/`, `Chain/`, `Session/`, `Log/`, `Test/`                     |
| `Health/`    | `Proxy/`, `Port/`                                                  |
| `Bin/`       | `SetupHelper.rs`                                                   |

## Phases

1. **Layout + naming** (parent, whole tree): done - `Source/`, PascalCase,
   full words, capitalised acronyms, `Cargo.toml` paths, `include_str!` paths,
   the packaging test's path normalisation, every `mod`/`use` path.
2. **Atomization** (one worker per module, disjoint ownership): done - every
   monolith split into its leaf tree, items renamed to their kind, the facade
   exporting the old name as an alias until phase 3.
3. **Alias sweep** (parent, single-threaded): done - all 189 compatibility
   re-exports deleted across 177 statements in 28 facades, every call site
   repointed at its leaf path. Final tree: `crates/aphrodite/Source/` 962
   files / 32,005 lines (largest 327), `crates/aphrodite-hermes/Source/` 244
   files / 6,525 lines (largest 149). `grep -rn 'TODO(house-style)' crates/`
   returns nothing.

## Alias rule (phase 2 only - closed in phase 3)

An item renamed to `Struct`/`Fn`/`Enum` keeps compiling for its callers
through one line in the owning facade:

```rust
pub use Struct::Struct as ProxyConfig; // TODO(house-style): drop after sweep
```

The alias is deleted in phase 3. Never edit a file you do not own - if a call
site outside your module needs a change, leave it to the sweep.

## Phase 4 - the code rewrite (after phase 3)

Phase 1-3 change where code lives. Phase 4 changes what the code says. It is a
**chain, not a wave**: each step rewrites identifiers or signatures the next
step depends on, so the order below is the order it runs in.

### 4.1 Identifiers inside the code

The file/directory rules already apply to every identifier:

- Types, fields, locals, parameters, constants, statics, generic parameters:
  PascalCase, full words, capitalised acronyms.
  `let requires_index_lock` → `let RequiresIndexLock`,
  `pub fn resolve_thresholds` → `pub fn Fn`, `TARGET_DIRS` → `static TargetDirectory`.
- Full words: `Configuration`, `Environment`, `Language`, `Number`,
  `Directory`, `Message`, `Buffer`, `Context`, `Argument`, `Parameter`,
  `Length`, `String`, `Error`, `Information`, `Manager`, `Specification`,
  `Utility`, `Temporary`, `Package`, `Repository`, `Dependency`.
- Acronyms: `URL`, `URI`, `ID`, `JSON`, `XML`, `HTML`, `CSV`, `YAML`, `JS`,
  `CLI`, `API`, `SDK`, `CCR`, `SSE`, `ABI`, `FFI`, `MCP`, `LLM`, `RPC`.
- Verbs are present-tense and singular: `Resolve`, `Parse`, `Render`, `Load`,
  `Store`, `Emit`, `Apply` - never `Resolved`, `Parsing`, `Parses`, `Actions`.

### 4.2 `pub use` removed everywhere

No facade re-exports. A consumer writes the full path
(`Preview::Builder::Build::Fn`), so the tree itself is the API and there is
exactly one way to name a thing. This also deletes the phase-2 compatibility
aliases as a class rather than one by one.

### 4.3 Branching becomes `match`; iteration becomes combinators

- `if`/`else if`/`else` chains → one `match` on the decision value.
- `if let Some(x) = ...` → `match ... { Some(x) => ..., None => ... }`.
- `for` loops → iterator combinators (`.for_each`, `.try_for_each`, `.map`,
  `.filter_map`) or `while let Some(x) = ...pop()`, whichever states the intent.
- `loop { ... }` with a `break` value → `match` on the terminal condition.
- Allowed, and only these: `if` as a `match` guard, `#[cfg(...)]`, and a
  const-context branch that `match` cannot express.

The reference crate (`PlayForm/Cargo/Run`) still contains `if`/`for`/labelled
`break`; this rule is deliberately stricter than the reference.

### 4.4 `anyhow` at every level

Every fallible function returns `anyhow::Result<T>`; every error crossing a
boundary gains `.context("...")` naming the operation and the input that failed.
`thiserror` types survive only where a caller matches on variants; everywhere
else they are wrapped. The C ABI keeps returning JSON error strings - `anyhow`
never crosses `extern "C"`.

### 4.5 Logging: `tracing` + `EnvFilter` at every level

Every crate initialises one subscriber with `EnvFilter` (from `RUST_LOG`, then
`APHRODITE_LOG`, then the configured level) and logs through `tracing` only:
`tracing::info_span!` around every entry point, `tracing::warn!`/`error!` for
degradation, `field::debug` for structured values. The hand-rolled
`Debug/mod.rs` logger and every `eprintln!` are removed or routed through
`tracing`. The subscriber initialises once per process, in `Main.rs` for the
binary and in the dylib's init for the plugin, and a second init is a no-op.

### 4.6 Parallelism where it pays

Copied from `PlayForm/Cargo/Run`'s `Fn/Binary/Command/Parallel.rs`:

- `rayon` `.par_iter()` / `.into_par_iter()` for the CPU-bound batch passes:
  preview building over a blob, `StructExtract` over lines, `Stage2` reducers,
  catalog formatting, directive loading.
- `crossbeam_queue::ArrayQueue` pre-sized to the work count, `Arc`-shared, with
  `rayon::current_num_threads()` workers, for the queue-shaped paths
  (`PollWorker`, `Prefetch`).
- `tokio` for the I/O-bound paths that already own a runtime (the proxy).
- `once_cell::sync::Lazy` for process-global state.

Bounded on purpose: never inside a lock, never on the per-hook path that runs
under Hermes' hook timeout, and never a pool sized per session - the plugin
lives in a shared process and the gateway-wedge history is the reason.

### 4.7 Vertical rhythm

One blank line between statements that do different things, between a statement
and a block, and between blocks. A run of closely related statements may stay
tight; two unrelated statements never share a line. `rustfmt` cannot enforce
this - it is a review rule, and `blank_lines_upper_bound = 1` in `rustfmt.toml`
keeps it from fighting the formatter.

## Verification (no new gates)

Correctness is proven by the existing build and test commands, not by new
scripts. Every worker runs, from the repo root:

```sh
cargo check -p <package> --all-targets
cargo test -p <package>
cargo clippy -p <package> -- -D warnings
```

and reports the real exit codes. The parent runs the workspace-wide set once
after each wave.
