//! # aphrodite — Chat Completions proxy wrapping headroom-core
//!
//! Aphrodite is a Rust proxy that sits between Hermes and DeepSeek's API,
//! providing CCR (Compress-Cache-Retrieve) for tool outputs in chat completions.
//!
//! ## Modes
//!
//! - **Cache** (default, :9797): In-memory CCR store, >8KB compression threshold,
//!   preserves a 512-char content preview. Lightweight, no tool injection.
//! - **Aphrodite** (:9798): SQLite-backed persistent CCR, >1KB threshold,
//!   aggressive compression with marker-only output, tool injection for
//!   `headroom_retrieve`, bidirectional tool relay endpoint.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/health` | Health check → `ok` |
//! | GET | `/stats` | Live proxy statistics |
//! | POST | `/retrieve` | Resolve CCR markers → original content |
//! | POST | `/tool/relay` | Execute tool calls through proxy |
//! | POST | `/ccr/create` | Programmatic CCR entry creation |
//! | GET | `/ccr/list` | List CCR entries |
//! | ANY | `/*path` | Proxy passthrough → DeepSeek |
//!
//! ## Quick Start
//!
//! ```bash
//! # Cache mode (default)
//! aphrodite --mode cache --listen 127.0.0.1:9797 --deepseek-key $KEY
//!
//! # Token mode (full CCR + tool relay)
//! aphrodite --mode aphrodite --listen 127.0.0.1:9798 --deepseek-key $KEY --tool-relay
//!
//! # Dev mode (verbose logging)
//! aphrodite --mode aphrodite --listen 127.0.0.1:9798 --deepseek-key $KEY --dev
//! ```
//!
//! ## Architecture
//!
//! ```text
//! Hermes → aphrodite (:9797/:9798) → DeepSeek API
//!              ↓ CCR store
//!         InMemoryCcrStore (cache) / SqliteCcrStore (aphrodite)
//!              ↓ Tool relay
//!         POST /tool/relay ← Hermes can call this
//! ```

pub mod config;
pub mod proxy;
pub mod retrieve;
