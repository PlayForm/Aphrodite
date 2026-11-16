//! Hook implementations - the compression decisions Aphrodite makes on a live
//! agent session.
//!
//! Every entry point here is called by the Hermes bridge, the proxy, or the C
//! ABI dispatch in `Library.rs` once per turn: tool results and terminal output
//! become CCR markers, the pre/post LLM hooks build context and archive the
//! turn, and the live-container path emits an instant marker for content the
//! agent should not wait on.
//!
//! Both transform paths have the same three steps, which is the shape of the
//! whole tree:
//!
//! 1. `Observation/` - what the call did, as a value, applied before any
//!    decision so no gate can drop it.
//! 2. `Classification/`, `Path/`, `Policy/`, `Select/`, `Plan/` - what to do
//!    about it, decided from values with no session mutation.
//! 3. `Apply/`, `Live/` - doing it, through the one shared `Emit/` leaf.
//!
//! `Reason/` is the vocabulary a passthrough reports and `Passthrough/` is the
//! one payload shape it emits; `AutoExpand/`, `Meta/`, `Hash/` and `Tool/` are
//! the small shared pieces; `Session/` and `LLM/` are the session lifecycle and
//! the per-call hooks.
//!
//! Behaviour is the contract: the compression thresholds, the live-chunking
//! byte arithmetic, the essential-tools bypass list and the exact JSON handed
//! to Hermes are all load-bearing, and the `Test/` battery pins them.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Hook::LLM::Pre::Fn::Fn`), so the file that defines an item is the only
//! place it is defined and there is exactly one way to spell it.

pub mod AutoExpand;
pub mod Classification;
pub mod Emit;
pub mod Hash;
pub mod LLM;
pub mod Live;
pub mod Meta;
pub mod Observation;
pub mod Outcome;
pub mod Passthrough;
pub mod Path;
pub mod Reason;
pub mod Session;
pub mod Terminal;
pub mod Tool;
pub mod ToolResult;

#[cfg(test)]
pub mod Test;
