//! `aphrodite_prefetch_status` - which prefetched files are resolvable now.
//!
//! Prefetch is synchronous here, so anything loaded is already "ready". Rendered
//! by the core crate's `Prefetch::prefetch_status`: when
//! `[compression] live_container` is off the output is byte-identical to the
//! original inline renderer; when on, ready entries additionally carry the
//! live-container arrival surface (arrived_at / bytes_arrived / eta_s).

use crate::Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_prefetch_status` handler.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_prefetch_status", |_Args| {
		crate::Session::With::Fn::with_shared(|State| aphrodite::Prefetch::Status::Fn::PrefetchStatus(State))
	})
}
