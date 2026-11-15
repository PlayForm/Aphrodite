//! The `const` constructor the static needs.

use std::sync::Mutex;

use super::Struct::Struct;

impl Struct {
	/// The empty table, usable in a `static` initialiser.
	///
	/// `Mutex::new(None)` and `Mutex::new(1)` are both `const`, which is why the
	/// map is stored as an `Option` and why the counter starts at 1: handle `0`
	/// is never handed out, so a zeroed or defaulted caller value cannot name a
	/// live session.
	pub(crate) const fn New() -> Self { Self { Sessions:Mutex::new(None), Next:Mutex::new(1) } }
}
