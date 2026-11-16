//! Resolving what to classify: the caller's hint against the unwrap guess.

use crate::Tool::{Compress::Classified, Unwrap};

/// What the preview and the type are computed from, for one payload.
///
/// The rules, in the order they apply:
///
/// 1. A non-default caller hint is INTENT and the unwrap is a GUESS, so the hint
///    wins whenever it is present. `"text"` is the schema's own default and is
///    treated as no hint at all.
/// 2. For a JSON payload the preview comes from the FULL content even when the
///    hint wins: an unwrap fragment (`ok`) previews as `[text:1L 2B | ok]`, which
///    counts the fragment instead of the payload and drops the shape signal.
/// 3. With no usable hint the unwrap's own body and type win; with no unwrap,
///    the shared classifier detects the type from the content.
///
/// It does not hash, store or render: the pair it returns is what those steps
/// consume, and the ORIGINAL content is what they must hash and store - `Body` is
/// only ever the preview's input.
pub(crate) fn Fn(Content:&str, Hint:&str) -> Classified::Struct::Struct {
	let Unwrapped = Unwrap::Fn::Fn(Content);
	let Hinted = (!Hint.is_empty() && Hint != "text").then_some(Hint);

	match (Unwrapped, Hinted) {
		(Some((Body, _)), Some(Hint)) => {
			// A JSON payload previews from itself, never from the fragment the
			// unwrapper extracted out of it.
			let Body = match Content.trim_start().starts_with('{') {
				true => Content.to_string(),
				false => Body,
			};

			Classified::Struct::Struct { Body, Kind:Hint.to_string() }
		},
		(Some((Body, Kind)), None) => Classified::Struct::Struct { Body, Kind },
		(None, Some(Hint)) => Classified::Struct::Struct { Body:Content.to_string(), Kind:Hint.to_string() },
		(None, None) => {
			Classified::Struct::Struct { Body:Content.to_string(), Kind:aphrodite::Preview::Detect::Type::Fn(Content) }
		},
	}
}
