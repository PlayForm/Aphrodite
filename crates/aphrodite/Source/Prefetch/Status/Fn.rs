//! The live prefetch schedule surface behind `aphrodite_prefetch_status`.
//!
//! Off-path output is byte-identical to the pre-port bridge renderer, and
//! that identity is a contract, not an accident: an agent that has been
//! conditioned on `{path, hash, type, size}` must not see a reshaped entry
//! because a feature flag was flipped. The live-container fields are added
//! alongside the base fields, never in place of them, and every meta value is
//! parsed defensively - a malformed or legacy entry degrades to zeroes rather
//! than failing the whole schedule.

use crate::State::Struct::AphroditeState;

/// Live prefetch schedule - the extended status struct behind the
/// `aphrodite_prefetch_status` tool (pre-Rust `_hooks/prefetch.py`
/// `_prefetch_status_handler` port).
///
/// When `state.live_container` is off the output is byte-identical to the
/// pre-port bridge renderer (aphrodite-hermes `Source/Tool/mod.rs`):
/// `loading` / `Ready` / `Errors` arrays plus `total_ready`, ready entries
/// carrying exactly `path` / `hash` / `type` / `Size`. When on, each ready
/// entry additionally carries the live-container arrival surface (`Live`,
/// `ArrivedAt`, `BytesArrived`, `EtaS`) and the top level gains
/// `live_container: true`; the base fields are untouched.
pub fn PrefetchStatus(State:&AphroditeState) -> serde_json::Value {
	let Live = State.LiveContainer;
	let Ready:Vec<serde_json::Value> = State
		.RecentMarkers
		.iter()
		.filter_map(|Mk| {
			let meta = Mk.meta.as_ref()?;
			let path = meta.get("path")?;
			let mut Entry = serde_json::json!({
				"path": path,
				"hash": &Mk.hash,
				"type": Mk.CCRType,
				"size": Mk.Size,
			});
			match Live {
				true => {
					// Defensive parse (meta values are strings): a malformed or
					// legacy entry degrades to "0 bytes arrived, 0s ETA" instead of
					// failing the whole schedule surface.
					let ArrivedAt = meta.get("live_arrived_at").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
					let BytesArrived = meta.get("live_bytes_arrived").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
					let EtaS = meta.get("live_eta_s").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
					Entry["live"] = serde_json::json!(meta.get("live").map(|v| v == "1").unwrap_or(false));
					Entry["arrived_at"] = serde_json::json!(ArrivedAt);
					Entry["bytes_arrived"] = serde_json::json!(BytesArrived);
					Entry["eta_s"] = serde_json::json!(EtaS);
				},
				false => (),
			}
			Some(Entry)
		})
		.collect();

	let mut Out = serde_json::json!({
		"loading": [],
		"ready": Ready,
		"errors": [],
		"total_ready": Ready.len(),
	});
	match Live {
		true => Out["live_container"] = serde_json::json!(true),
		false => (),
	}
	Out
}
