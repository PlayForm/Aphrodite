//! Resolve a search envelope into the payload it carries.

use serde_json::{Map, Value};

use crate::Tool::Unwrap::Search::{Payload, Rows};

/// The payload a search envelope carries, or `None` when it is not a search.
///
/// `total_count` is the cheap guard and it comes FIRST, before a single row is
/// rebuilt: an object without it is not a search envelope, and normalising its
/// `matches` would be work done for a payload that is about to be rejected.
///
/// The rows form wins when either row shape produced something; otherwise the
/// count-only form is only admitted for a GENUINE search shape. A data object
/// that merely carries a `total_count` key (e.g. `{"total_count": 42,
/// "items": [...]}`) is not a search and must preview as itself, so it returns
/// `None` rather than a label.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<Payload::Enum::Enum> {
	let Count = Object.get("total_count").and_then(Value::as_u64)?;
	let Truncated = Object.get("truncated").and_then(Value::as_bool).unwrap_or(false);

	let FromArray = Rows::Array::Fn::Fn(Object);
	let Found = (!FromArray.is_empty())
		.then_some(FromArray)
		.or_else(|| Rows::Text::Fn::Fn(Object));

	match Found {
		Some(Found) => Some(Payload::Enum::Enum::Rows(Found)),
		None => {
			let IsSearch = Object.get("matches").is_some()
				|| Object.get("matches_text").is_some()
				|| Object.get("matches_format").is_some()
				|| Truncated;

			IsSearch.then_some(Payload::Enum::Enum::Total { Count, Truncated })
		},
	}
}
