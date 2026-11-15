//! `note_root_session`: publish the root session id.

use crate::Debug::Root::Const::RootSession;

/// Publish the root session id (Hermes bridge `record_session`).
///
/// An empty id is ignored rather than stored: the bridge calls this on every
/// session start including subagents that carry no root, and storing "" would
/// make `LogPath` answer `None` for a session that HAS a root - the log would
/// silently go nowhere.
///
/// The store is conditional on the id actually changing. Re-publishing the same
/// id would be a write every session start for a value that already matches, and
/// `then` makes that a comparison rather than a store.
pub fn Fn(Root:&str) {
	let _ = (!Root.is_empty()).then(|| {
		let mut Guard = RootSession
			.get_or_init(|| std::sync::Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		let _ = (Guard.is_empty() || *Guard != Root).then(|| *Guard = Root.to_string());
	});
}
