---
name: aphrodite-cargo-upgrade
description:
    "Cargo upgrade breakpoints for aphrodite + headroom - reqwest features, axum
    ConnectInfo+fallback, tokio-tungstenite Messages, pyo3 allow_threads,
    workspace pinning. What breaks and how to fix."
version: 1.0.0
platforms: [macos]
---

# Cargo Upgrade Breakpoints

After `cargo upgrade` (via `~/Developer/Maintain/Fn/Update/Cargo.sh`), verify
these before assuming clean build.

## Reqwest 0.12 → 0.13

### Feature rename: `rustls-tls` → `rustls`

**Symptom**:
`package depends on reqwest with feature rustls-tls but reqwest does not have that feature`

**Fix**: Replace `rustls-tls` with `rustls` in all Cargo.toml files. Check both
`[dependencies]` AND `[dev-dependencies]` - they can have different feature
strings:

```toml
# Before
reqwest = { features = ["stream", "rustls-tls", "http2"] }
reqwest = { features = ["stream", "rustls-tls", "http2", "json"] }  # dev-deps
# After
reqwest = { features = ["stream", "rustls", "http2"] }
reqwest = { features = ["stream", "rustls", "http2", "json"] }
```

## Axum 0.7 → 0.8

### Wildcard routes

**Symptom**: `Path segments must not start with *` at startup

**Fix**: `.route("/*path", ...)` → `.route("/{*path}", ...)`

### ConnectInfo + fallback handlers

**Symptom**:
`the trait bound fn(State<AppState>, ConnectInfo<...>, ...) -> ... {catch_all}: Handler<_, _> is not satisfied`

**Cause**: `any(catch_all)` with `ConnectInfo<SocketAddr>` extractor fails the
Handler trait bound in axum 0.8's stricter `fallback()`.

**Fix**: Pin axum to 0.7 in workspace Cargo.toml.

## Tokio-Tungstenite 0.24 → 0.29

### Message type changes

**Symptom**: `mismatched types: expected Bytes, found Vec<u8>` and
`expected Utf8Bytes, found String`

**Fix**: All Message variants now use `Bytes`/`Utf8Bytes` instead of
`Vec<u8>`/`String`:

```rust
// Before (0.24)
TgMsg::Ping(p) => AxMsg::Ping(p.to_vec()),
TgMsg::Pong(p) => AxMsg::Pong(p.to_vec()),
TgMsg::Binary(b) => AxMsg::Binary(b.to_vec()),
AxMsg::Text(t) => TgMsg::Text(t.to_string()),

// After (0.29)
TgMsg::Ping(p) => AxMsg::Ping(p),           // Bytes → Bytes
TgMsg::Pong(p) => AxMsg::Pong(p),           // Bytes → Bytes
TgMsg::Binary(b) => AxMsg::Binary(b),       // Bytes → Bytes
AxMsg::Text(t) => TgMsg::Text(t.to_string().into()),  // → Utf8Bytes
TgMsg::Text(t) => AxMsg::Text(t.as_str().to_string().into()),
```

## PyO3 0.24 → 0.29

### allow_threads removed

**Symptom**: `no method named allow_threads found for struct pyo3::Python`

**Fix**: Major migration needed. Pin pyo3 to 0.24 until migration is done.

## SHA2 0.10 → 0.11

**Symptom**: `LowerHex is not satisfied` on `Array<u8, ...>`

**Fix**:

```rust
// Before
let hex = format!("{:x}", digest);
// After
let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
```

## Workspace vs Crate Pinning

When a workspace-level dep upgrade breaks individual crates, pin at crate level:

```toml
# headroom-proxy/Cargo.toml - pin locally, overrides workspace
tokio-tungstenite = { version = "0.24", ... }
```

Cargo resolves crate-local versions independently.

## ExpandVersions.rs

**Warning**: `Document` deprecated → `DocumentMut` in toml_edit. Cosmetic only.

```rust
use toml_edit::{DocumentMut, Item, Value};  // was Document
```

## Verification

```bash
cargo check -p aphrodite
cd vendor/headroom && cargo check # default-members only (skips headroom-py)
```
