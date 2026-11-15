//! `CurrentRootSession`: the in-process id, or the persisted one.

use crate::Debug::{Path::Runtime::Fn::Fn as DebugRuntimeHome, Root::Const::RootSession};

/// Resolve the current root session id: the in-process static, falling back
/// to the `session.current` file the bridge persists across restarts.
///
/// The file fallback is what makes a freshly started process log into the
/// SAME per-session file as the process that started it; dropping it would
/// scatter one session's history across one file per process. An empty result
/// is a legitimate answer (no session yet) - the caller decides what that
/// means, not this function.
pub fn Fn() -> String {
	{
		let Guard = RootSession
			.get_or_init(|| std::sync::Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);
		if !Guard.is_empty() {
			return Guard.clone();
		}
	}
	std::fs::read_to_string(DebugRuntimeHome().join("session.current"))
		.ok()
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}
