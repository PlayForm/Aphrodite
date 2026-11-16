//! Store the root session id, when it changed.

use crate::Debug::Root::Const::RootSession;

/// Publish `Root` into the process-wide slot, and nothing else.
///
/// The store is conditional on the id actually changing: re-publishing the same
/// id would be a write every session start for a value that already matches, and
/// `LogPath` reads this slot on every log line.
pub fn Fn(Root:&str) {
	let mut Guard = RootSession
		.get_or_init(|| std::sync::Mutex::new(String::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);

	(*Guard != Root)
		.then_some(Root)
		.into_iter()
		.for_each(|Root| Guard.replace_range(.., Root));
}
