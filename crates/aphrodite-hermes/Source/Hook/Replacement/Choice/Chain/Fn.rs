//! The chain-split replacement.

use serde_json::Value;

/// The NATURAL per-segment markers of a chain split, joined one per line.
///
/// Invisibility contract: the model sees exactly what it sees for any compressed
/// output - each segment's own `<<<CCR:hash|type|size>>>` marker, retrievable by
/// its own hash. Never the payload's `summary` string, which announces the
/// mechanism (`[chain:N segs | ...]`) and stays in the JSON for telemetry only.
///
/// `None` covers both "this was not a chain split" and "a chain split that
/// produced no marker": a join of zero markers is not a replacement, so it must
/// fall through to the next outcome rather than replace the output with nothing.
pub(crate) fn Fn(Result:&Value) -> Option<String> {
	Result
		.get("chain_split")
		.and_then(Value::as_bool)
		.unwrap_or(false)
		.then(|| {
			Result
				.get("markers")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(|Marker| Marker.get("marker").and_then(Value::as_str))
				.collect::<Vec<&str>>()
				.join("\n")
		})
		.filter(|Joined| !Joined.is_empty())
}
