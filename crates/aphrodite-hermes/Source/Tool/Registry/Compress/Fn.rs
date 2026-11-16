//! `aphrodite_compress` - store content, return a resolvable marker.

use crate::Tool::{Arg, Compress, Dispatch::Fn::Handler};

/// The `aphrodite_compress` handler.
///
/// An empty `content` is rejected rather than stored: a zero-byte entry would
/// hash to the same value as every other empty payload and shadow it in the
/// inline store.
pub(crate) fn Fn() -> (&'static str, Handler) {
	("aphrodite_compress", |Args| {
		let Content = Arg::Fn::Fn(Args, "content");

		let Some(Content) = (!Content.is_empty()).then_some(Content) else {
			return serde_json::json!({ "error": "content is required" });
		};

		let Hint = Arg::Fn::Fn(Args, "type");
		let Center = Args.get("_ccr_center").and_then(|v| v.as_str());

		crate::Session::With::Fn::with_shared(|State| Compress::Fn::Fn(State, Content, Hint, Center))
	})
}
