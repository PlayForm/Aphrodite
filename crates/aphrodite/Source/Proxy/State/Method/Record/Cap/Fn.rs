//! Keep a ring buffer at its bound.

use std::collections::VecDeque;

/// Drop the oldest entries until at most `Limit` remain.
///
/// A ring is bounded by construction here rather than by the caller remembering
/// to pop after every push: the excess is one number, and removing it is one
/// operation.
pub fn Fn<T>(Ring:&mut VecDeque<T>, Limit:usize) {
	let Excess = Ring.len().saturating_sub(Limit);

	Ring.drain(..Excess);
}
