//! `LogPath`: the log file for the current session.

use std::path::PathBuf;

use crate::Debug::{Path::ForRoot::Fn::Fn as LogPathForRoot, Root::Current::Fn::Fn as CurrentRootSession};

/// The per-session debug log path; `None` until a root session is known
/// (the bridge publishes it on the first `PreLLMCall`).
///
/// `None` rather than a fallback filename: an unnamed log would be shared by
/// every session in the process and would corrupt the legacy flag file that
/// lives in the same place. Silence until a root is known is the honest answer,
/// and `then` is what expresses it - the empty id produces no path rather than a
/// path that has to be rejected downstream.
pub(crate) fn Fn() -> Option<PathBuf> {
	let Root = CurrentRootSession();

	(!Root.is_empty()).then(|| LogPathForRoot(&Root))
}
