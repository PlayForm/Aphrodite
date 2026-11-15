//! aphrodite - Reverse proxy with Chat Completions API support.
//!
//! Two modes:
//! - **Cache** (:9797): In-memory CCR, lightweight compression, no tool
//!   injection. Passes most content through, only compresses very large outputs
//!   (>8KB).
//! - **Token** (:9798): SQLite CCR, aggressive compression, tool injection,
//!   tool relay for bidirectional Hermes communication.
//!
//! Chat Completions API:
//! - Forwards POST /v1/chat/completions to DeepSeek
//! - Intercepts responses, compresses tool output via CCR
//! - Does NOT inject the aphrodite_retrieve tool definition into response
//!   tool_calls (that was tried and reverted - see Bug 18 in
//!   `Compress::Chat`); the Python plugin registers the tool instead.
//!
//! This file is a facade and carries no logic. The tree below is grouped by
//! function, one item per file, each item named by its kind - so a reader
//! looking for the threshold arithmetic lands on `State::Method::Threshold`
//! instead of scrolling a monolith. Consumers name the leaf path directly, so
//! the file that defines an item is the only place it is defined.

pub mod Body;
pub mod CCR;
pub mod Cache;
pub mod Compress;
pub mod Detect;
pub mod Format;
pub mod Handle;
pub mod Marker;
pub mod Metadata;
pub mod Relay;
pub mod Secret;
pub mod State;
pub mod Stream;
pub mod Threshold;
pub mod Upstream;

// `pub(crate)` (not private) so other in-crate test modules - e.g.
// `Retrieve::tests` - can reach `test_state_with_ccr()` without duplicating
// `AppState`'s ~47-field literal (report 05 T5 verification).
#[cfg(test)]
pub(crate) mod Test;
