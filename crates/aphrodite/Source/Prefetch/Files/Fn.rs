//! The one-call convenience path: read then insert, back to back.
//!
//! This is the shape callers use when they already hold the state lock for
//! the whole operation. It is not the shape to reach for when the point is to
//! keep I/O out of the lock - that caller composes `Paths` and
//! `Insert` itself, which is exactly why the split exists.

use crate::{
	Prefetch::{Insert::Fn::Fn as Insert, Read::Entry::Fn::Fn as Paths},
	State::Struct::AphroditeState,
};

/// Prefetch a list of file paths into the inline store.
/// Returns JSON with status per file: loaded, skipped (too large), missing.
///
/// Convenience wrapper: does the read and the insert back-to-back, still
/// requiring `State` (and thus whatever lock guards it) for the whole call.
/// Callers that already hold a lock across the whole operation (like this
/// crate's own tests, and `aphrodite-hermes`'s `with_shared`) can keep using
/// this directly. Callers that want to read files *before* taking their
/// lock - the point of this split - call `Paths` then `Insert`
/// separately; see the `AphroditeDispatch` "prefetch" arm in `Library.rs`.
pub fn Fn(State:&mut AphroditeState, List:&[String]) -> serde_json::Value { Insert(State, Paths(List)) }
