//! Releasing a handle.

use super::Struct::{Session, Struct};

impl Struct {
	/// Drop the session named by `Hid` from the table.
	///
	/// What removal means, precisely: it prevents every future lookup, and it
	/// does **not** cancel a call already in flight. A caller that resolved the
	/// handle before this ran holds a cloned `Arc`, so its operation finishes
	/// safely against state that is still alive; the state itself is freed when
	/// the last `Arc` drops. A destroyed handle therefore never revives the
	/// session, and never truncates a concurrent one.
	pub(crate) fn Remove(&self, Hid:usize) -> Option<Session> { self.With(|Map| Map.remove(&Hid)) }
}
