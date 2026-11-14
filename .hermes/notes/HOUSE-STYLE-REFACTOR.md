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
   `SSE`, `ABI`, `FFI`.
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

- **C ABI leaves keep their exported identifier.** `Source/Abi/Compress.rs`
  defines `pub extern "C" fn aphrodite_compress`, not `fn Fn`. The symbol
  name IS the ABI contract that `aphrodite-hermes`, the ctypes bindings and
  `Maintain/check_ffi_contract.py` assert; renaming it would break codegen
  (cbindgen → `_bindings.py`) and the plugin. One export per file still holds.
- **Struct field names stay `snake_case`.** Several structs are `serde`-
  serialised onto the proxy wire (`ToolRelayRequest`, `CcrNotification`, the
  configuration tables). Renaming fields changes JSON a third-party client
  sees - a wire break, not a style change.
- **`BuiltinDirective/` keeps lowercase `.md` filenames.** The filenames are
  the directive identifiers (`focus`, `foresight`, `ccr-handling`) surfaced to
  the model and to `aphrodite_directive("add", ...)`; the directory is
  PascalCase, the data filenames are keys.

## Naming applied in phase 1 (done)

`src/` → `Source/`, `lib.rs` → `Library.rs`, `main.rs` → `Main.rs`,
`tests.rs` → `Test.rs`, `builtin_directives/` → `BuiltinDirective/`.

| Old                    | New                            | Old                 | New                    |
| ---------------------- | ------------------------------ | ------------------- | ---------------------- |
| `catalog.rs`           | `Catalog/`                     | `hooks.rs`          | `Hook/`                |
| `chain_split.rs`       | `ChainSplit/`                  | `marker/`           | `Marker/`              |
| `config_loader.rs`     | `ConfigurationLoader/`         | `poll_worker.rs`    | `PollWorker/`          |
| `config/`              | `Configuration/`               | `prefetch.rs`       | `Prefetch/`            |
| `config/cli.rs`        | `Configuration/CLI.rs`         | `preview/builders/` | `Preview/Builder/`     |
| `config/env.rs`        | `Configuration/Environment.rs` | `preview/detectors/`| `Preview/Detector/`    |
| `debug.rs`             | `Debug/`                       | `preview/line/`     | `Preview/Line/`        |
| `directives/`          | `Directive/`                   | `preview/text/`     | `Preview/Text/`        |
| `flow.rs`              | `Flow/`                        | `preview/lang.rs`   | `Preview/Language.rs`  |
| `home.rs`              | `Home/`                        | `preview/type.rs`   | `Preview/Type.rs`      |
| `proxy.rs`             | `Proxy/`                       | `resolve/`          | `Resolve/`             |
| `retrieve.rs`          | `Retrieve/`                    | `setup/`            | `Setup/`               |
| `scripting.rs`         | `Scripting/`                   | `stage2.rs`         | `Stage2/`              |
| `session.rs`           | `Session/`                     | `state/`            | `State/`               |
| `struct_extract/`      | `StructExtract/`               | `templates.rs`      | `Template/`            |
| `hermes/schemas.rs`    | `Schema/`                      | `hermes/tools.rs`   | `Tool/`                |
| `hermes/directives.rs` | `Directive/`                   | `hermes/bin/`       | `Bin/`                 |

Acronym leaves: `JSON`, `XML`, `HTML`, `CSV`, `YAML`, `JS`, `YAMLKey`,
`EnvelopeJSON`, `MarkdownHeading`, `MarkdownStructure`, `NumberBefore`.

## Module map - the target trees (phase 2)

### aphrodite

| Module                 | Leaves                                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Library.rs`           | crate docs, `pub mod`, `pub use Preview::{build_preview, detect_type}`                                              |
| `Abi/`                 | one file per `aphrodite_*` export; `Handle/{Table,Allocate,Lookup}.rs`; `Guard.rs`                                   |
| `Main.rs`              | `fn main()` only                                                                                                    |
| `Boot/`                | `Router/`, `Listen/`, `Shutdown/`, `Configuration/`                                                                  |
| `Guard/`               | `Loopback/`, `Host/`, `Bearer/`, `Token/`                                                                            |
| `Proxy/`               | `State/`, `Threshold/`, `Body/`, `Cache/`, `CCR/`, `Relay/`, `Metadata/`, `Marker/`, `Test/`                         |
| `Hook/`                | `Live/`, `ToolResult/`, `Terminal/`, `Session/`, `Llm/`, `Path/`, `Test/`                                            |
| `ConfigurationLoader/` | `Config/`, `Warn/`, `Test/`                                                                                          |
| `PollWorker/`          | `Task/`, `Should/`, `Check/`, `Update/`, `Insert/`, `Status/`, `Test/`                                                |
| `Flow/`                | `Prompt/`, `Nudge/`, `Signature/`, `Window/`, `TurnContext/`, `Test/`                                                 |
| `Retrieve/`            | `Live/`, `Page/`, `Filter/`, `Request/`, `Response/`, `Test/`                                                         |
| `Session/`             | `Start/`, `Turn/`, `Archive/`, `Index/`, `Catalog/`, `Engine/`, `Test/`                                               |
| `Prefetch/`            | `Arrival/`, `Read/`, `Outcome/`, `Insert/`, `Status/`, `Test/`                                                        |
| `Template/`            | `Vars/`, `Extract/`, `Render/`, `Set/`, `Get/`, `Family/`, `Test/`                                                    |
| `Stage2/`              | one reducer per file, `Registry/`, `Compress/`, `Test/`                                                               |
| `Debug/`               | `Level/`, `Log/`, `Counter/`, `Timing/`, `Metrics/`, `Path/`, `Test/`                                                 |
| `Catalog/`             | `Build/`, `Format/`, `Size/`, `Test/`                                                                                 |
| `Scripting/`           | `Engine/`, `Script/`, `Payload/`, `Test/`                                                                             |
| `ChainSplit/`          | `Segment/`, `Split/`, `MarkedCommand/`, `MarkedOutput/`, `ErrorHint/`, `Test/`                                         |
| `Home/`                | `Hermes/`, `Runtime/`, `Plugin/`, `Environment/`, `Tilde/`, `Path/`, `Test/`                                          |
| `State/`               | `Struct/`, `Default/`, `Const/`, `Template/` + `ChainSplit/`, `Directive/`, `Event/`, `File/`, `Marker/`, `Store/`     |
| `Directive/`           | `Action/`, `Build/`, `Builtin/`, `Load/`, `Model/`, `Test/`                                                            |
| `Marker/`              | `Format/`, `Parse/`, `Test/`                                                                                          |
| `Preview/`             | `Builder/`, `Detector/`, `Line/`, `Text/`, `Input/`, `Language/`, `Type/`, `Detect/`, `State/`, `Test/`                |
| `Resolve/`             | `Expand/`, `Filter/`, `One/`, `Parse/`, `Recursive/`, `Test/`                                                          |
| `Setup/`               | `Dylib/`, `Write/`, `Run/`, `Verify/`, `Macos/`, `Test/`                                                               |
| `StructExtract/`       | `Detect/`, `Format/`, `Go/`, `JS/`, `Python/`, `Rust/`, `Test/`                                                        |
| `Configuration/`       | `CLI/`, `Environment/`, `Proxy/`, `Test/`                                                                              |

### aphrodite-hermes

| Module       | Leaves                                                          |
| ------------ | --------------------------------------------------------------- |
| `Library.rs` | crate docs, `pub mod`, re-exports                               |
| `Session/`   | `Registry/`, `Lock/`, `With/`, `Broadcast/`, `Watcher/`          |
| `Abi/`       | one file per `aphrodite_hermes_*` export                        |
| `Hook/`      | `FailOpen/`, `Replacement/`, `PostTool/`, `Log/`                 |
| `Tool/`      | `Registry/`, `Dispatch/`, `Read/`, `Unwrap/`, `Compress/`, `Test/` |
| `Schema/`    | one file per tool schema, `All/`, `Get/`, `Test/`                 |
| `Directive/` | `Resolve/`, `Builtin/`, `Materialize/`, `Test/`                   |
| `Debug/`     | `Flag/`, `Chain/`, `Session/`, `Log/`, `Test/`                    |
| `Health/`    | `Proxy/`, `Port/`                                                 |
| `Bin/`       | `SetupHelper.rs`                                                  |

## Phases

1. **Layout + naming** (parent, whole tree): done - `Source/`, PascalCase,
   full words, capitalised acronyms, `Cargo.toml` paths, `include_str!` paths,
   the packaging test's path normalisation, every `mod`/`use` path.
2. **Atomization** (one worker per module, disjoint ownership): split each
   monolith into its leaf tree, rename items to their kind, keep the facade
   exporting the old name as an alias until phase 3.
3. **Alias sweep** (parent, single-threaded): drop every compatibility
   `pub use Old as New;`, update the remaining call sites, re-gate.

## Alias rule (phase 2 only)

An item renamed to `Struct`/`Fn`/`Enum` keeps compiling for its callers
through one line in the owning facade:

```rust
pub use Struct::Struct as ProxyConfig; // TODO(house-style): drop after sweep
```

The alias is deleted in phase 3. Never edit a file you do not own - if a call
site outside your module needs a change, leave it to the sweep.

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
