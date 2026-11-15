//! `note_root_session`: publish the root session id.

use crate::Debug::Root::Const::RootSession;

/// Publish the root session id (Hermes bridge `record_session`).
///
/// An empty id is ignored rather than stored: the bridge calls this on every
/// session start including subagents that carry no root, and storing "" would
/// make `LogPath` answer `None` for a session that HAS a root - the log would
/// silently go nowhere.
pub fn Fn(Root:&str) {
	if Root.is_empty() {
		return;
	}
	let mut Guard = RootSession
		.get_or_init(|| std::sync::Mutex::new(String::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	if Guard.is_empty() || *Guard != Root {
		*Guard = Root.to_string();
	}
}
