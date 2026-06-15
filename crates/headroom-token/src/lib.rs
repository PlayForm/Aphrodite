//! aphrodite — Chat Completions proxy with CCR, tool relay, and programmatic CCR.
//!
//! Two modes:
//! - **Cache** (:9797): In-memory CCR, lightweight compression, preview preserved.
//! - **Token** (:9798): SQLite CCR, aggressive compression, tool injection,
//!   tool relay for bidirectional Hermes communication.

pub mod config;
pub mod proxy;
pub mod retrieve;
