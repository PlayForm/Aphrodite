//! The one way into the map.

use std::collections::HashMap;

use super::Struct::{Session, Struct};

impl Struct {
	/// Run `Operation` against the map under the table lock, initialising the
	/// map on first use.
	///
	/// The guard never leaves this function: a consumer gets the answer, not the
	/// lock, which is what makes "the outer mutex is held for a map operation
	/// and nothing else" a property of the type rather than a rule every caller
	/// re-earns. A poisoned lock is recovered rather than propagated - one
	/// panicking hook must not make every later call on every other handle fail.
	pub(super) fn With<T>(&self, Operation:impl FnOnce(&mut HashMap<usize, Session>) -> T) -> T {
		let mut Guard = self.Sessions.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

		Operation(Guard.get_or_insert_with(HashMap::new))
	}
}
