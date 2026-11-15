//! The live-container fields of one ready entry.

use std::collections::HashMap;

use serde_json::Value;

/// The live-container surface of a ready entry, when the feature is on.
///
/// Added ALONGSIDE the base fields, never in place of them: an agent conditioned
/// on `{path, hash, type, size}` must not see a reshaped entry because a flag was
/// flipped. Every meta value is parsed defensively - a malformed or legacy entry
/// degrades to "0 bytes arrived, 0s ETA" rather than failing the whole schedule,
/// because a status surface that throws is worse than one that under-reports.
pub(crate) fn Fn(Meta:&HashMap<String, String>) -> Vec<(&'static str, Value)> {
	let ArrivedAt = Meta
		.get("live_arrived_at")
		.and_then(|Value| Value.parse::<u64>().ok())
		.unwrap_or(0);
	let BytesArrived = Meta
		.get("live_bytes_arrived")
		.and_then(|Value| Value.parse::<u64>().ok())
		.unwrap_or(0);
	let EtaS = Meta
		.get("live_eta_s")
		.and_then(|Value| Value.parse::<f32>().ok())
		.unwrap_or(0.0);
	let Live = Meta.get("live").map(|Value| Value == "1").unwrap_or(false);

	vec![
		("live", serde_json::json!(Live)),
		("arrived_at", serde_json::json!(ArrivedAt)),
		("bytes_arrived", serde_json::json!(BytesArrived)),
		("eta_s", serde_json::json!(EtaS)),
	]
}
