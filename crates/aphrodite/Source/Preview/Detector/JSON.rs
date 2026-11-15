//! JSON shape detector.
//!
//! A strict parse is the strongest shape signal available, so this detector runs
//! FIRST in the chain: a JSON payload can never be hijacked by a marker-shaped
//! substring inside one of its string values. It is not a schema check - any
//! parseable object or array is JSON.

use serde_json::Value as JSONValue;

use crate::Preview::{Input::Struct::Struct as Input, Text::EnvelopeJSON};

/// True when the content parses as a JSON object or array.
///
/// Hermes wrapper envelopes (`Output`/`ExitCode`, `Diff`, `error`, `Success`,
/// `TotalCount`/`Matches`, `Content`/`total_lines`, `Result`/`Message`/`Found`/
/// `Preview`, `name`/`description`) are EXCLUDED: their raw-JSON previews are
/// deliberately the caller-visible payload, and hiding an error message behind a
/// key listing would be a regression. It is not a validator of the payload's
/// meaning, only of its syntax.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	if let Ok(v) = serde_json::from_str::<JSONValue>(Inp.raw) {
		match v {
			JSONValue::Object(Obj) => {
				if !EnvelopeJSON::Fn(&Obj) {
					return true;
				}
			},
			JSONValue::Array(_) => return true,
			_ => {},
		}
	}
	false
}
