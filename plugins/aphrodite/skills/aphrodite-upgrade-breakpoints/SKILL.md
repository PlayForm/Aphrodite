---
name: aphrodite-upgrade-breakpoints
description: "Cargo upgrade breakpoints checklist — axum 0.8 wildcards, sha2 0.11 hex, reqwest features. What breaks and how to fix it."
version: 1.0.0
platforms: [macos]
---

# Cargo Upgrade Breakpoints

After `cargo upgrade` (via `~/Developer/Maintain/Fn/Update/Cargo.sh`), verify these before assuming clean build:

## Axum 0.7 → 0.8

**Symptom**: `Path segments must not start with *` at startup

**Fix**: `crates/aphrodite/src/main.rs` — one-line change:
```rust
// Before
.route("/*path", any(proxy::proxy_handler))
// After
.route("/{*path}", any(proxy::proxy_handler))
```

## SHA2 0.10 → 0.11

**Symptom**: `LowerHex is not satisfied` on `Array<u8, ...>`

**Affects**: 4 sites in `vendor/headroom/crates/headroom-core/src/transforms/`

**Fix**:
```rust
// Before
let hex = format!("{:x}", digest);
// After
let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
```

## Reqwest Features

`rustls-tls` → `rustls` in Cargo.toml.

## Verification
```bash
cargo check --release -p aphrodite
cd vendor/headroom && cargo test -p headroom-core --lib -- ccr
```
