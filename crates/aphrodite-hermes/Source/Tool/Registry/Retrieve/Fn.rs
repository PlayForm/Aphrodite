//! `aphrodite_retrieve` - resolve a CCR hash (recursively) or read a path.

use Tool::Arg;
use Tool::Dispatch::Fn::ToolHandler;
use Tool::Read;

/// The `aphrodite_retrieve` handler.
///
/// `path` wins when both arguments are present, and a path read never touches the
/// session lock - it is a disk read, and holding the state mutex across it would
/// stall every concurrent hook.
///
/// The `query` filter runs on the resolved body, so it narrows what reaches
/// context without changing what is stored.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_retrieve", |args| {
		let path = Arg::Fn::Fn(args, "path");
		if !path.is_empty() {
			return match Read::Fn::Fn(path) {
				Ok(content) => {
					let query = Arg::Fn::Fn(args, "query");
					let body = if query.is_empty() {
						content
					} else {
						aphrodite::Resolve::Filter::Fn(&content, query)
					};
					serde_json::json!({"found": true, "source": "path", "path": path, "content": body})
				},
				Err(e) => serde_json::json!({"found": false, "error": e}),
			};
		}

		let hash = Arg::Fn::Fn(args, "hash");
		if hash.is_empty() {
			return serde_json::json!({"error": "hash or path is required"});
		}
		let query = Arg::Fn::Fn(args, "query").to_string();
		crate::Session::With::Fn::with_shared(|state| {
			match aphrodite::Resolve::Expand::Fn(state, hash) {
				Some(content) => {
					let body = if query.is_empty() {
						content
					} else {
						aphrodite::Resolve::Filter::Fn(&content, &query)
					};
					serde_json::json!({"found": true, "source": "ccr", "hash": hash, "content": body})
				},
				None => serde_json::json!({"found": false, "hash": hash, "error": "hash not found in session store"}),
			}
		})
	})
}
