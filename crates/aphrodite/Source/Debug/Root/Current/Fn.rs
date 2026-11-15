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
///
/// The guard's block is load-bearing: it must be released before the fallback
/// runs, or the file read happens while the session-id mutex is held. `or_else`
/// on a value the block already produced keeps that boundary visible - the
/// closure cannot capture `Guard` because `Guard` no longer exists by then.
pub fn Fn() -> String {
	let InProcess = {
		let Guard = RootSession
			.get_or_init(|| std::sync::Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		(!Guard.is_empty()).then(|| Guard.clone())
	};

	InProcess.unwrap_or_else(|| {
		std::fs::read_to_string(DebugRuntimeHome().join("session.current"))
			.ok()
			.map(|s| s.trim().to_string())
			.unwrap_or_default()
	})
}
