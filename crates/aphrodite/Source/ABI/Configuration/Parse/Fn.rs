//! `Parse`: a key and a value as a typed write, or the reason it is not.

use super::super::{Outcome::Enum::Enum as Outcome, Update::Enum::Enum as Update};

/// Turn the caller's key and value into a write, or into the reason there is
/// none.
///
/// Acceptance rules, preserved exactly from the inline match this replaces:
///
/// - `model` takes any string, including the empty one.
/// - `engine_threshold_pct` and `tool_threshold` accept a decimal integer and
///   **ignore** an unparseable one rather than rejecting it - `Invalid`,
///   not an error. Tightening that is a behaviour change and belongs to its own
///   change, not to this one.
/// - `context_engine_enabled` accepts the literal `"true"` or `"1"` and reads
///   anything else as false. It does not accept `"yes"`, `"on"` or a case
///   variant, and it has never answered an error for one.
/// - Every other key, `api_url` included, is `Unknown`.
///
/// Nothing here touches session state, which is the point: the whole parse runs
/// before the session lock is taken.
pub(crate) fn Fn(Key:&str, Value:&str) -> Result<Update, Outcome> {
	match Key {
		"model" => Ok(Update::Model(Value.to_string())),
		"engine_threshold_pct" => Value.parse().map(Update::Engine).map_err(|_| Outcome::Invalid),
		"tool_threshold" => Value.parse().map(Update::Tool).map_err(|_| Outcome::Invalid),
		"context_engine_enabled" => Ok(Update::Context(Value == "true" || Value == "1")),
		_ => Err(Outcome::Unknown),
	}
}
