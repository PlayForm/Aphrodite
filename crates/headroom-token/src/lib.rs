//! headroom-token — Standalone token-mode proxy with CCR + SQLite storage.
//!
//! Forwards requests to DeepSeek, compresses tool output with headroom-core
//! CCR, stores compressed content in SQLite (persistent), and exposes
//! a `/retrieve` endpoint without governor auth.

pub mod config;
pub mod proxy;
pub mod retrieve;
