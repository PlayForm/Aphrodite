//! The registry's fields.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

use crate::State::Struct::AphroditeState;

/// One session's state behind its own lock, as the table stores it.
///
/// A type alias, not a wrapper: the table's value is exactly "that session's
/// mutex" and a newtype around it would be a second name for the same thing
/// with nothing to enforce. The invariant that matters - the outer table lock
/// is only ever held long enough to look this up and clone it, never across a
/// hook or tool body - is enforced by `With`, which is the only way in.
pub(crate) type Session = Arc<Mutex<AphroditeState>>;

/// The process-global handle table: a caller's handle to that session's state.
///
/// `Sessions` is `None` until the first allocation so the static stays `const`
/// without requiring a heap allocation at load time; `With` initialises it on
/// first use and no consumer sees the `Option`.
///
/// `Next` is the id counter. It is a separate mutex from `Sessions` because
/// allocation must not serialise lookups: a session resolving its own handle
/// while another session is being created is the common case at startup.
///
/// Each session's state lives behind its own `Arc<Mutex<..>>` (F4/T6, report
/// 06) rather than directly in the map: previously one global lock serialized
/// every session's every call - two different agent sessions could not
/// classify/compress concurrently at all, even though they share nothing.
pub(crate) struct Struct {
	pub(super) Sessions:Mutex<Option<HashMap<usize, Session>>>,
	pub(super) Next:Mutex<usize>,
}
