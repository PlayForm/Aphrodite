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
// Phase 4.3 rewrites every `if`/`if let` into a `match` on the decision value,
// which is the house rule; these two lints argue the opposite, so they are
// allowed here exactly as they already are in `Library.rs`.
#![allow(clippy::match_bool)]
#![allow(clippy::single_match)]

mod Boot;
mod Guard;

fn main() -> anyhow::Result<()> { Boot::Fn::Fn() }
