//! `aphrodite_retrieve` - resolve a CCR hash (recursively) or read a path.

use crate::Tool::{
	Arg,
	Dispatch::Fn::Handler,
	Read,
	Registry::Retrieve::{Narrow, Target},
};

/// The `aphrodite_retrieve` handler.
///
/// `path` wins when both arguments are present (`Target::Resolve` owns that rule,
/// and the target is a value rather than a pair of booleans), and a path read
/// never touches the session lock - it is a disk read, and holding the state mutex
/// across it would stall every concurrent hook.
///
/// Three outcomes stay distinct, because a caller that cannot tell them apart
/// reads a refusal as "the content is gone": no argument at all is
/// `{"error": ...}`, a read or lookup that failed is `{"found": false, "error":
/// ...}`, and a hit is `{"found": true, "source": ..., "content": ...}`.
///
/// The `query` narrowing runs on the RESOLVED body, so it narrows what reaches
/// context without changing what is stored.
pub(crate) fn Fn() -> (&'static str, Handler) {
	("aphrodite_retrieve", |Args| {
		let Some(Resolved) = Target::Resolve::Fn::Fn(Args) else {
			return serde_json::json!({ "error": "hash or path is required" });
		};

		match Resolved {
			// A path read is a disk read: it must never reach `with_shared`.
			Target::Enum::Enum::Path(Path) => {
				match Read::Fn::Fn(Path) {
					Ok(Content) => {
						serde_json::json!({
							"found": true,
							"source": "path",
							"path": Path,
							"content": Narrow::Fn::Fn(Content, Arg::Fn::Fn(Args, "query")),
						})
					},
					Err(e) => serde_json::json!({ "found": false, "error": format!("{e:#}") }),
				}
			},
			Target::Enum::Enum::Hash(Hash) => {
				let Query = Arg::Fn::Fn(Args, "query").to_string();

				crate::Session::With::Fn::with_shared(|State| {
					match aphrodite::Resolve::Expand::Fn(State, Hash) {
						Some(Content) => {
							serde_json::json!({
								"found": true,
								"source": "ccr",
								"hash": Hash,
								"content": Narrow::Fn::Fn(Content, &Query),
							})
						},
						None => {
							serde_json::json!({ "found": false, "hash": Hash, "error": "hash not found in session store" })
						},
					}
				})
			},
		}
	})
}
