//! Hook implementations - the compression decisions Aphrodite makes on a live
//! agent session.
//!
//! Every entry point here is called by the Hermes bridge, the proxy, or the C
//! ABI dispatch in `Library.rs` once per turn: tool results and terminal output
//! become CCR markers, the pre/post LLM hooks build context and archive the
//! turn, and the live-container path emits an instant marker for content the
//! agent should not wait on.
//!
//! The tree is grouped by WHAT is being transformed, not by call order:
//!
//! - `ToolResult/` - tool-result compression: entry points, the gate sequence,
//!   telemetry recording, written-path inference, type resolution.
//! - `Terminal/` - terminal-output compression, including the chain-split path
//!   for output that already carries segment markers.
//! - `Live/` - the live-container store: chunking, keys, the instant marker.
//! - `AutoExpand/` - the `[compression] auto_expand` consumer.
//! - `Meta/`, `Hash/`, `Tool/`, `Path/` - the small shared pieces.
//! - `Session/`, `LLM/` - the session lifecycle and the per-call hooks.
//!
//! Behaviour is the contract: the compression thresholds, the live-chunking
//! byte arithmetic, the essential-tools bypass list and the exact JSON handed
//! to Hermes are all load-bearing, and the `Test/` battery pins them.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Hook::LLM::Pre::Fn::Fn`), so the file that defines an item is the only
//! place it is defined and there is exactly one way to spell it.

pub mod AutoExpand;
pub mod Hash;
pub mod LLM;
pub mod Live;
pub mod Meta;
pub mod Path;
pub mod Session;
pub mod Terminal;
pub mod Tool;
pub mod ToolResult;

#[cfg(test)]
mod Test;

// ── Public surface (called from `Library.rs`, `Proxy/`, `Retrieve/` and the
// `aphrodite-hermes` bridge) ───────────────────────────────────────────────

pub use Terminal::Entry::Fn::{
	TransformTerminalOutput,
	TransformTerminalOutputClassified,
	TransformTerminalOutputWithMeta,
};
pub use ToolResult::Entry::Fn::{TransformToolResult, TransformToolResultClassified, TransformToolResultWithMeta};
