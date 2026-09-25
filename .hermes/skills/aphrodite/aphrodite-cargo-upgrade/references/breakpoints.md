# Known Dependency Breakpoints

Symptom → fix notes for the dependencies that have already broken during past
upgrades. Each entry names the live manifest that currently protects it and the
fact class it belongs to (rewrite.md: **historical observation** = explanatory;
**source-derived** = re-verify in the checked-out manifest before acting).

Manifest state below verified 2026-09-20; re-derive from the named file, never
from this note, when a bump is in flight.

## reqwest 0.12 → 0.13: feature rename `rustls-tls` → `rustls`

**Symptom**:
`package depends on reqwest with feature rustls-tls but reqwest does not have that feature`

**Fix**: Replace `rustls-tls` with `rustls` in every Cargo.toml that declares
reqwest features. Check BOTH `[dependencies]` AND `[dev-dependencies]` - they
can carry different feature strings:

```toml
# Before
reqwest = { features = ["stream", "rustls-tls", "http2"] }
reqwest = { features = ["stream", "rustls-tls", "http2", "json"] }  # dev-deps
# After
reqwest = { features = ["stream", "rustls", "http2"] }
reqwest = { features = ["stream", "rustls", "http2", "json"] }
```

**Live state**: `crates/aphrodite/Cargo.toml` is on reqwest `0.13.5`
(default-features = false) - this breakpoint is in force there.
`vendor/headroom` (workspace) and its crates are still on reqwest `0.12` with
the old feature strings; do not "normalize" them to `rustls` until headroom is
actually bumped to 0.13.

## axum 0.7 → 0.8

### Wildcard routes

**Symptom**: `Path segments must not start with *` at startup.

**Fix**: `.route("/*path", ...)` → `.route("/{*path}", ...)`.

### ConnectInfo + fallback handlers

**Symptom**:
`the trait bound fn(State<AppState>, ConnectInfo<...>, ...) -> ... {catch_all}: Handler<_, _> is not satisfied`

**Cause**: `any(catch_all)` with `ConnectInfo<SocketAddr>` extractor fails the
Handler trait bound in axum 0.8's stricter `fallback()`.

**Fix**: Pin axum to 0.7 in the workspace Cargo.toml (do not migrate the
fallback handler until the route table is on 0.8).

**Live state**: `vendor/headroom/Cargo.toml` workspace pins `axum = "0.7"`
(headroom-proxy uses it with `ws`, `http2`, `macros`); `crates/aphrodite` is on
`axum 0.8.9` with the `{*path}` syntax - the split is deliberate and protects
the proxy's ConnectInfo fallback.

## tokio-tungstenite 0.24 → 0.29: Message type changes

**Symptom**: `mismatched types: expected Bytes, found Vec<u8>` and
`expected Utf8Bytes, found String`.

**Fix**: All Message variants now use `Bytes`/`Utf8Bytes` instead of
`Vec<u8>`/`String`:

```rust
// Before (0.24)
TgMsg::Ping(p) => AxMsg::Ping(p.to_vec()),
TgMsg::Pong(p) => AxMsg::Pong(p.to_vec()),
TgMsg::Binary(b) => AxMsg::Binary(b.to_vec()),
AxMsg::Text(t) => TgMsg::Text(t.to_string()),

// After (0.29)
TgMsg::Ping(p) => AxMsg::Ping(p),                        // Bytes → Bytes
TgMsg::Pong(p) => AxMsg::Pong(p),                        // Bytes → Bytes
TgMsg::Binary(b) => AxMsg::Binary(b),                    // Bytes → Bytes
AxMsg::Text(t) => TgMsg::Text(t.to_string().into()),     // → Utf8Bytes
TgMsg::Text(t) => AxMsg::Text(t.as_str().to_string().into()),
```

**Live state**: `vendor/headroom/crates/headroom-proxy/Cargo.toml` pins
`tokio-tungstenite = "0.24"` in both `[dependencies]` and `[dev-dependencies]`

- a live crate-local pin, recorded as such. Only 0.24's `Vec<u8>`/`String`
  arms are currently compiled.

## PyO3 0.24 → 0.29: allow_threads removed

**Symptom**: `no method named allow_threads found for struct pyo3::Python`.

**Fix**: Major migration needed (GIL-release calls must move to the 0.29
replacement API). Historically the guidance was "pin pyo3 to 0.24 until the
migration is done".

**Live state**: the migration has LANDED - `vendor/headroom/Cargo.toml`
workspace pins `pyo3 = { version = "0.29", features = ["abi3-py310"] }` and
headroom-py inherits it via `workspace = true`. If a future bump re-raises the
symptom, classify it as an **ABI/FFI change** (Step 4), not a routine rename:
the FFI contract checker and `test_finalize_bindings.py` are the gates.

## SHA2 0.10 → 0.11: LowerHex removed

**Symptom**: `LowerHex is not satisfied` on `Array<u8, ...>`.

**Fix**:

```rust
// Before
let hex = format!("{:x}", digest);
// After
let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
```

**Live state**: SPLIT graph, verified: `vendor/headroom/crates/headroom-core`
is on `sha2 = "0.11"` (hex migration applied); `headroom-proxy` and
`vendor/rtk` are still on `sha2 = "0.10"`. Byte parity with Python
`hashlib.sha256` (marker/hash formatting) is the acceptance test - see
`_hash_field_name` in smart_crusher.

## Workspace vs crate pinning

When a workspace-level dep upgrade breaks one crate, pin at crate level:

```toml
# headroom-proxy/Cargo.toml - pin locally, overrides workspace
tokio-tungstenite = { version = "0.24", ... }
```

Cargo resolves crate-local versions independently of the workspace default.
A local pin MUST be recorded in the compatibility-pin ledger: it can hide a
split dependency graph (`cargo tree -p <crate>` shows one version, the rest of
the workspace another) and increases the behavioral-test surface (both
versions' runtime paths).

## toml_edit Document → DocumentMut (ExpandVersions.rs helper)

**Symptom**: deprecation of `toml_edit::Document` in the version-expansion
helper script (cosmetic only).

**Fix**:

```rust
use toml_edit::{DocumentMut, Item, Value};  // was Document
```

## Silent runtime breakage (absorbed from aphrodite-upgrade-breakpoints)

Compilation-clean refactors that break at runtime - these are why Step 6
(behavioral tests) is mandatory:

- **tracing `DisplayValue<T>`** requires `fmt::Display`; `PathBuf` must use
  `.display()`, not the `%` format specifier.
- **`_headroom_context` import** comes from `.health`, NOT `.env` - importing
  from the wrong module silently kills the plugin load path.

Provenance: rebuild-path resolution (`_find_cargo_toml`), the
`--version`-before-config-load check, and the standalone plugin repo behavior
were absorbed into `aphrodite-operations` on 2026-09-18; this file holds only
the dependency/runtime breakpoints.
