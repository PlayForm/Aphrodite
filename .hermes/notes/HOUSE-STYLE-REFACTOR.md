# House-Style Refactor - Aphrodite crates

Target: both crates under `crates/` adopt the PlayForm Rust house style
(`PlayForm/Cargo/Run`, `Dotology`) - one item per file, item named by kind,
deeply nested PascalCase modules, `mod.rs` facades. Package identity is
unchanged: `crates/aphrodite` stays `aphrodite`, `crates/aphrodite-hermes`
stays `aphrodite-hermes`, and every `libaphrodite_hermes.dylib` path, CI
alias, release asset and plugin loader reference stays valid.

## The contract

1. **Crate root is `Source/Library.rs`** (lib) and `Source/Main.rs` (bin).
   `Cargo.toml` carries `[lib] path = "Source/Library.rs"` and
   `[[bin]] path = "Source/Main.rs"`. `Library.rs` holds crate docs, the
   `#![allow(non_snake_case)]` gate, module declarations and re-exports -
   nothing else.
2. **A module is a PascalCase directory** with a `mod.rs` facade listing
   `pub mod <Leaf>;` one per line. A module that is a single unit today
   (`hooks.rs`) becomes a directory (`Hook/`) even before it is split.
3. **A leaf file owns exactly one item and names it by kind**:
   `Struct` (struct), `Enum` (enum), `Fn` (function), `Trait`, `Const`.
   The file and directory names carry the domain; the item name carries the
   kind. Consumers path to it: `Preview::Builder::Build::Fn`,
   `State::Marker::Struct`.
4. **Deep nesting is the default.** A leaf that is a group of related items
   (`Proxy/Body/Request` with 445 lines of trait impl) becomes another
   directory, never a 400-line file. Practical ceiling: a leaf file over
   ~150 lines is a signal to nest once more.
5. **Facades re-export, they do not define.** A facade may `pub use` a
   child's item when the child owns the module's canonical item
   (`Token/mod.rs` → `pub use Token::Struct;`), and must not carry logic.
6. **Tests stay with their module**: `<Module>/Test.rs`, declared
   `#[cfg(test)] mod Test;` in the facade. A test file over ~200 lines
   splits by area (`<Module>/Test/<Area>.rs`).
7. **Formatting**: rustfmt with the repo's `rustfmt.toml` (hard tabs,
   `space_after_colon = false`, `max_width = 120`).

### Deviations, and why

- **C ABI leaves keep their exported identifier.** `Source/Abi/Compress.rs`
  defines `pub extern "C" fn aphrodite_compress`, not `fn Fn`. The symbol
  name IS the ABI contract that `aphrodite-hermes`, the ctypes bindings and
  `Maintain/check_ffi_contract.py` assert; renaming it would break codegen
  (cbindgen → `_bindings.py`) and the plugin. One export per file still
  holds.
- **Struct field names stay `snake_case`.** Several structs are
  `serde`-serialised onto the proxy wire (`ToolRelayRequest`,
  `CcrNotification`, the config tables). Renaming fields to PascalCase
  changes the JSON that a third-party client sees - a wire break, not a
  style change.
- **`BuiltinDirective/` keeps lowercase `.md` filenames.** The filenames are
  the directive identifiers (`focus`, `foresight`, `ccr-handling`) surfaced
  to the model and to `aphrodite_directive("add", ...)`; the directory is
  PascalCase, the data filenames are keys.

## Module map (aphrodite crate)

`src/` → `Source/`, `lib.rs` → `Library.rs`, `main.rs` → `Main.rs`,
`builtin_directives/` → `BuiltinDirective/`.

| Old path                          | New path                            | Split into                                                                 |
| --------------------------------- | ----------------------------------- | -------------------------------------------------------------------------- |
| `lib.rs`                          | `Library.rs` + `Abi/`               | one file per `aphrodite_*` export, `Abi/Handle/{Table,Alloc,Get}.rs`       |
| `main.rs`                         | `Main.rs` + `Boot/` + `Guard/`      | `Boot/{Router,Listen,Shutdown,Config}.rs`, `Guard/{Loopback,Host,Bearer}.rs` |
| `proxy.rs`                        | `Proxy/`                            | `State/`, `Threshold/`, `Body/`, `Cache/`, `Ccr/`, `Relay/`, `Metadata/`   |
| `hooks.rs`                        | `Hook/`                             | `Live/`, `ToolResult/`, `Terminal/`, `Session.rs`, `Llm.rs`, `Path.rs`     |
| `config_loader.rs`                | `ConfigLoader/`                     | `Config/` (per method group), `Warn.rs`                                    |
| `poll_worker.rs`                  | `PollWorker/`                       | `Task/`, `Should/`, `Check.rs`, `Update.rs`, `Insert.rs`, `Status.rs`      |
| `flow.rs`                         | `Flow/`                             | `Prompt/`, `Nudge/`, `Signature/`, `Window/`, `TurnContext/`               |
| `retrieve.rs`                     | `Retrieve/`                         | `Live/`, `Page/`, `Filter.rs`, `Request.rs`, `Response.rs`                 |
| `session.rs`                      | `Session/`                          | `Turn.rs`, `Archive.rs`, `Index.rs`, `Catalog.rs`, `Engine.rs`, `Start.rs`  |
| `prefetch.rs`                     | `Prefetch/`                         | `Arrival/`, `Read/`, `Outcome.rs`, `Insert.rs`, `Status.rs`                |
| `templates.rs`                    | `Template/`                         | `Vars/`, `Extract/`, `Render.rs`, `Set.rs`, `Get.rs`, `Family.rs`          |
| `stage2.rs`                       | `Stage2/`                           | one reducer per file + `Registry.rs`, `Compress.rs`                        |
| `debug.rs`                        | `Debug/`                            | `Level/`, `Log/`, `Counter.rs`, `Timing.rs`, `Metrics.rs`, `Path.rs`       |
| `catalog.rs`                      | `Catalog/`                          | `Build.rs`, `Format.rs`, `Size.rs`                                         |
| `scripting.rs`                    | `Scripting/`                        | `Engine/`, `Script.rs`, `Payload.rs`                                       |
| `chain_split.rs`                  | `ChainSplit/`                       | one file per fn + `Segment.rs`                                             |
| `home.rs`                         | `Home/`                             | `Hermes.rs`, `Runtime.rs`, `Plugin.rs`, `Env.rs`, `Tilde.rs`, `Path.rs`    |
| `state/mod.rs`                    | `State/`                            | `Struct.rs`, `Default.rs`, `Const.rs`, `Template.rs` + existing submodules |
| `directives/`                     | `Directive/`                        | existing 5 leaves, PascalCase, `Test.rs`                                   |
| `marker/`                         | `Marker/`                           | `Format/`, `Parse/`, `Test.rs`                                             |
| `preview/`                        | `Preview/`                          | `Builder/`, `Detector/`, `Line/`, `Text/` + existing leaves                |
| `resolve/`                        | `Resolve/`                          | existing leaves, PascalCase, `Test.rs`                                     |
| `setup/`                          | `Setup/`                            | `Dylib/`, `Write/`, `Run/`, `Verify.rs`, `Macos.rs`                        |
| `struct_extract/`                 | `StructExtract/`                    | existing leaves, PascalCase, `Test.rs`                                     |
| `config/`                         | `Config/`                           | `Proxy/` (one struct per file), `Cli/`, `Env.rs`, `Test.rs`                |

## Module map (aphrodite-hermes crate)

| Old path              | New path                                  | Split into                                                             |
| --------------------- | ----------------------------------------- | ---------------------------------------------------------------------- |
| `lib.rs`              | `Library.rs` + `Session/` + `Abi/`        | `Session/{Registry,Lock,With,Broadcast,Watcher}.rs`, `Abi/` per export  |
| `tools.rs`            | `Tool/`                                   | `Registry/`, `Dispatch.rs`, `Read.rs`, `Unwrap.rs`, `Compress.rs`       |
| `schemas.rs`          | `Schema/`                                 | one file per tool schema + `All.rs`, `Get.rs`                           |
| `directives.rs`       | `Directive/`                              | `Resolve.rs`, `Builtin.rs`, `Materialize/`                              |
| `debug.rs`            | `Debug/`                                  | `Flag/`, `Chain/`, `Session/`, `Log.rs`                                 |
| `bin/setup-helper.rs` | `Bin/SetupHelper.rs`                      | unchanged body, `[[bin]] path` updated                                  |

## Migration phases

1. **Layout + naming** (parent, whole tree, one commit-shaped change):
   `src/` → `Source/`, `lib.rs` → `Library.rs`, `main.rs` → `Main.rs`,
   PascalCase every module file and directory, update `Cargo.toml` paths,
   `include_str!` paths, the packaging test's path normalisation and every
   `mod`/`use` path. Gate: `cargo check --workspace --all-targets`,
   `cargo test --workspace`, `cargo clippy -p aphrodite -p aphrodite-hermes
   -- -D warnings`, `cargo fmt --all -- --check`.
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

The alias is deleted in phase 3. Never leave an alias in a file the worker
does not own.

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
