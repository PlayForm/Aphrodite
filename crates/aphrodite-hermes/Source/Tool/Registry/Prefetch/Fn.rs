//! `aphrodite_prefetch` - read and store files now, returning one marker each.

use crate::Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_prefetch` handler.
///
/// F4/T5 (report 06): reads happen BEFORE the `with_shared` lock is taken, not
/// inside it - the session state is a single mutex shared by every hook and tool
/// call, so disk I/O held under it (network/USB-mounted paths, slow filesystems)
/// stalls every other concurrent hook for the duration of the read.
///
/// An empty `paths` array is rejected rather than returning an empty success: a
/// prefetch that loaded nothing is a caller mistake, and silence would read as
/// success.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_prefetch", |args| {
		let paths:Vec<String> = args
			.get("paths")
			.and_then(|v| v.as_array())
			.map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
			.unwrap_or_default();
		let Some(paths) = (!paths.is_empty()).then_some(paths) else {
			return serde_json::json!({ "error": "paths is required" });
		};

		let outcomes = aphrodite::Prefetch::Read::Entry::Fn::ReadPaths(&paths);
		crate::Session::With::Fn::with_shared(|state| aphrodite::Prefetch::Insert::Fn::InsertOutcomes(state, outcomes))
	})
}
