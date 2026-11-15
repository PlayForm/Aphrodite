//! Resolving a handle.

use super::Struct::{Session, Struct};

impl Struct {
	/// Clone a session's `Arc` out from under the table lock.
	///
	/// The clone is what lets the lock be dropped before the caller locks the
	/// session: the map lock is never held across state access, so two sessions
	/// run concurrently and a slow hook on one never serialises the other.
	pub(crate) fn Resolve(&self, Hid:usize) -> Option<Session> { self.With(|Map| Map.get(&Hid).cloned()) }
}
