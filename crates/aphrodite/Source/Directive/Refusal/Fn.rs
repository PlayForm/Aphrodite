//! Render a directive refusal as the one error value.

use super::Enum::Enum;

/// The error value for one refusal.
///
/// One function because the reply shape is one shape: both `AphroditeDirective` and
/// `AphroditeDispatch`'s `"directive"` arm pass this value straight through their success
/// serializer (01-F8), so an error is an object with an `error` key and nothing else - never a
/// second envelope a caller has to recognise.
///
/// The unknown-action message lists the vocabulary because it is the only documentation a model
/// gets at the moment it gets it wrong.
pub(crate) fn Fn(Refusal:&Enum, Value:&str) -> serde_json::Value {
	let Message = match Refusal {
		Enum::UnknownAction => format!("unknown action: {Value} (use list|swap|add|load|remove|reset)"),
		Enum::UnknownDirective => format!("unknown directive: {Value}"),
	};

	serde_json::json!({ "error": Message })
}
