//! Minting a handle.

use std::sync::Mutex;

use super::Struct::Struct;
use crate::State::Struct::AphroditeState;

impl Struct {
	/// Allocate a fresh handle for `State` and insert it into the table.
	///
	/// The returned ID is what the caller receives as a decimal string. Collision
	/// policy: ids come from a counter, so two live handles can only collide
	/// after `usize` wraps, and the counter wraps rather than saturating because
	/// a saturated counter would hand out one ID forever and silently alias every
	/// later session onto the first one's state. On that (unreachable) wrap the
	/// insert replaces the stale entry, which is the same aliasing the saturation
	/// would produce - the wrap at least lets it self-correct.
	///
	/// The counter lock is released before the table lock is taken, so a lookup
	/// on another thread is never blocked behind an allocation.
	pub(crate) fn Allocate(&self, State:AphroditeState) -> usize {
		let Hid = {
			let mut Next = self.Next.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			let Hid = *Next;

			*Next = Next.wrapping_add(1); // overflow-safe

			Hid
		};

		self.With(|Map| Map.insert(Hid, std::sync::Arc::new(Mutex::new(State))));

		Hid
	}
}
