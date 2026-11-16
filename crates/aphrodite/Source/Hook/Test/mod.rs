//! The Hook test battery, split by transformation path.
//!
//! Each area file proves one path end to end: the payload shape, the marker
//! grammar, what landed in the store, and the resolve round trip. They are the
//! contract for the compression thresholds, the live-chunking byte arithmetic,
//! the essential-tool bypass list and the exact JSON Hermes receives - a
//! change that breaks one of them is a behaviour change, not a refactor.

mod Live;
mod Session;
mod Terminal;
mod ToolResult;

use crate::{Hook::Hash::Fn::Fn as ComputeHash, State::Struct::AphroditeState};

/// Store `Text` under its own CCR hash and return the hash.
///
/// The auto-expand tests need a nested entry whose marker text is embedded in
/// the content under test, so they can prove the hook actually resolves
/// instead of passing the wrapper through.
pub(super) fn StoreNested(State:&mut AphroditeState, Text:&str) -> String {
	let h = ComputeHash(Text);
	State.Store.Put(h.clone(), Text.to_string());
	h
}
