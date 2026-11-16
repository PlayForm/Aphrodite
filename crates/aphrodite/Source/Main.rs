//! aphrodite - Multi-proxy LLM proxy with CCR + tool relay.
//!
//! Two modes:
//! 1. Single proxy: `aphrodite --mode cache --listen :9797 --api-key <key>`
//! 2. Multi-proxy: `Aphrodite` (reads aphrodite.toml, spawns all listeners)
//!
//! This is the binary root and nothing else: `fn main` is the whole file. The
//! proxy - its router, its guards, its shutdown path - lives under `Boot` and
//! `Guard`. Keeping the entry point to one delegating call means the `[[bin]]
//! path` in `Cargo.toml` never has to move again, however the boot pipeline is
//! restructured.

#![allow(non_snake_case)]
// Constants and statics are PascalCase house-wide; see `Library.rs`.
#![allow(non_upper_case_globals)]
// A two-branch decision is a value, not a branch; see `Library.rs`.
#![allow(clippy::obfuscated_if_else)]

mod Boot;
mod Guard;

fn main() -> anyhow::Result<()> { Boot::Fn::Fn() }
