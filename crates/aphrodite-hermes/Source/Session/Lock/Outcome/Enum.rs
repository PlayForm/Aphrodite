//! What one bounded lock acquisition produced.

use std::sync::MutexGuard;

use aphrodite::State::Struct::AphroditeState;

/// The outcome of one bounded acquisition, named.
///
/// `Option` carried the same two states but left the second one to a comment, so
/// a caller could fold "the lock is wedged by an abandoned worker" into whatever
/// else its own `None` meant - which is exactly how a sweep that could not look
/// at any session reported the same value as a sweep that looked and found
/// nothing to do. The variants make the two situations different types, so a
/// caller that wants to count them can, and a caller that only wants to degrade
/// has to say which of the two it is degrading.
///
/// There is no third variant: `try_lock` reports only "acquired" or "would
/// block", and poison is recovered into a guard (see `Lock::Fn::Fn`), so a
/// poisoned session is a session this crate can still read.
pub(crate) enum Enum<'a> {
	/// Acquired. The guard is the only way to reach the session's state.
	Held(MutexGuard<'a, AphroditeState>),
	/// The deadline expired with the mutex still held elsewhere: an abandoned
	/// worker owns it. The caller MUST degrade - never block, never retry
	/// unboundedly.
	Wedged,
}
