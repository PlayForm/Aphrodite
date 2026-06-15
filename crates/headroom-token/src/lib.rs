//! headroom-proxy — Standalone proxy with cache + token modes, CCR + SQLite storage.
//!
//! Forwards requests to DeepSeek, compresses tool output with headroom-core
//! CCR, stores compressed content in SQLite (persistent), and exposes
//! endpoints for retrieval, tool relay, and programmatic CCR management.
//!
//! Two modes:
//! - Cache (:9797): standard compression, no tool injection.
//! - Token (:9798): aggressive compression, CCR, tool injection,
//!   tool relay for bidirectional Hermes communication,
//!   programmatic CCR with notification callbacks.

pub mod config;
pub mod proxy;
pub mod retrieve;
