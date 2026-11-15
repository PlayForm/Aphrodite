//! The shared `AppState`: its type, its constructor, its compression
//! helpers and its method groups.
//!
//! One `Arc<AppState>` is cloned into every handler, so this directory is the
//! single source of truth for what the proxy knows at runtime - configuration,
//! live-tunable atomics, counters and the mutex-guarded caches.

pub mod Build;
pub mod CCR;
pub mod Compress;
pub mod Method;
pub mod Struct;
