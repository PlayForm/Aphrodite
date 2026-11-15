//! The shared `AppState`: its type, its constructor, its method groups.
//!
//! One `Arc<AppState>` is cloned into every handler, so this directory is the
//! single source of truth for what the proxy knows at runtime - configuration,
//! live-tunable atomics, counters and the mutex-guarded caches.
//!
//! `CCR` holds the spawn_blocking wrappers for the store's read, count and
//! delete operations. Writes are not here: they belong to `Storage`, which is
//! the only thing that can produce the evidence a marker is built from.

pub mod Build;
pub mod CCR;
pub mod Method;
pub mod Struct;
