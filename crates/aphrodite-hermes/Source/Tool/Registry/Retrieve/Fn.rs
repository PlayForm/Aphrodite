//! `aphrodite_retrieve` - resolve a CCR hash (recursively) or read a path.

use crate::Tool::{Arg, Dispatch::Fn::ToolHandler, Read};

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
		let hash = Arg::Fn::Fn(args, "hash");

		match (path.is_empty(), hash.is_empty()) {
			// A path read is a disk read: it must never reach `with_shared`.
			(false, _) => {
				match Read::Fn::Fn(path) {
					Ok(content) => {
						let query = Arg::Fn::Fn(args, "query");
						let body = match query.is_empty() {
							true => content,
							false => aphrodite::Resolve::Filter::Fn(&content, query),
						};

						serde_json::json!({ "found": true, "source": "path", "path": path, "content": body })
					},
					Err(e) => serde_json::json!({ "found": false, "error": format!("{e:#}") }),
				}
			},
			(true, true) => serde_json::json!({ "error": "hash or path is required" }),
			(true, false) => {
				let query = Arg::Fn::Fn(args, "query").to_string();

				crate::Session::With::Fn::with_shared(|state| {
					match aphrodite::Resolve::Expand::Fn(state, hash) {
						Some(content) => {
							let body = match query.is_empty() {
								true => content,
								false => aphrodite::Resolve::Filter::Fn(&content, &query),
							};

							serde_json::json!({ "found": true, "source": "ccr", "hash": hash, "content": body })
						},
						None => {
							serde_json::json!({ "found": false, "hash": hash, "error": "hash not found in session store" })
						},
					}
				})
			},
		}
	})
}
