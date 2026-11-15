//! The one-call convenience path: read then insert, back to back.
//!
//! This is the shape callers use when they already hold the state lock for
//! the whole operation. It is not the shape to reach for when the point is to
//! keep I/O out of the lock - that caller composes `ReadPaths` and
//! `InsertOutcomes` itself, which is exactly why the split exists.

use crate::{
	Prefetch::{Insert::Fn::InsertOutcomes, Read::Entry::Fn::ReadPaths},
	State::AphroditeState,
};

/// Prefetch a list of file paths into the inline store.
/// Returns JSON with status per file: loaded, skipped (too large), missing.
///
/// Convenience wrapper: does the read and the insert back-to-back, still
/// requiring `State` (and thus whatever lock guards it) for the whole call.
/// Callers that already hold a lock across the whole operation (like this
/// crate's own tests, and `aphrodite-hermes`'s `with_shared`) can keep using
/// this directly. Callers that want to read files *before* taking their
/// lock - the point of this split - call `ReadPaths` then `InsertOutcomes`
/// separately; see the `AphroditeDispatch` "prefetch" arm in `Library.rs`.
pub fn PrefetchFiles(State:&mut AphroditeState, Paths:&[String]) -> serde_json::Value {
	InsertOutcomes(State, ReadPaths(Paths))
}
