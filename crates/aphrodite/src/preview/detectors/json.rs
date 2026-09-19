//! JSON shape detector - runs FIRST in the chain (strict parse is the
//! strongest shape signal; a JSON payload can never be hijacked by a marker
//! substring inside one of its string values).

use serde_json::Value as JsonValue;

use crate::preview::input::Input;
use crate::preview::text::envelope_json::is_envelope_json_object;

/// Strict parse of an object/array. Hermes wrapper envelopes
/// (`output`/`exit_code`, `diff`, `error`, `success`, `total_count`/`matches`,
/// `content`/`total_lines`, `result`/`message`/`found`/`preview`,
/// skill_view `name`/`description`) are EXCLUDED: their raw-JSON previews are
/// deliberately the caller-visible payload (WS1 full-content preview), and
/// hiding e.g. an error message behind a key listing would be a regression.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	if let Ok(v) = serde_json::from_str::<JsonValue>(inp.raw) {
		match v {
			JsonValue::Object(obj) => {
				if !is_envelope_json_object(&obj) {
					return true;
				}
			},
			JsonValue::Array(_) => return true,
			_ => {},
		}
	}
	false
}