//! ChainSplit - fine-grained decomposition of chained shell commands.
//!
//! LLMs habitually chain commands (`cd x && cargo build && cargo test`) into
//! one terminal call. The resulting single output blob defeats per-type
//! compression: it is one `Build`-typed entry, retrieved wholesale, and the
//! agent's context pays the full cost whenever it wants any of it.
//!
//! This module rewrites such chains by inserting segment markers between the
//! component commands, and splits the produced output back into per-segment
//! pieces. `pre_tool_call` rewrites the command; `TransformToolResult`
//! detects the markers and compresses each segment independently, so the
//! agent sees N compact per-segment previews (`[chain:3 | build ... ]`) and
//! retrieves exactly the segment it needs.
//!
//! Safety: markers are plain `echo` statements that never alter command
//! semantics (exit codes, streams, side effects). A chain with zero or one
//! segment, or containing a marker-resisting construct (heredoc, single
//! quotes spanning the separator, trailing backslash continuation), is left
//! untouched.
//!
//! The tree is grouped by the two directions of one format: `Split/` parses a
//! command into segments, `Marked/` writes the marker rewrite and reads the
//! marked output back apart, `Segment/` is the item they pass around,
//! `Const.rs` is the marker's exact bytes, and `ErrorHint/` is the
//! content-level hint attached to a failed segment.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`ChainSplit::Split::Fn::Fn`), so the file that defines an item is the only
//! place it is defined and there is exactly one way to spell it.

pub mod Const;
pub mod ErrorHint;
pub mod Marked;
pub mod Segment;
pub mod Split;

#[cfg(test)]
pub mod Test;
