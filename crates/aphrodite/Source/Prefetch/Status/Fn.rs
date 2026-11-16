//! The live prefetch schedule surface behind `aphrodite_prefetch_status`.
//!
//! Off-path output is byte-identical to the pre-port bridge renderer, and that
//! identity is a contract, not an accident: an agent that has been conditioned on
//! `{path, hash, type, size}` must not see a reshaped entry because a feature flag
//! was flipped. The live-container fields are added alongside the base fields,
//! never in place of them, and every meta value is parsed defensively - a
//! malformed or legacy entry degrades to zeroes rather than failing the whole
//! schedule.

use crate::{Prefetch::Status::Live::Fn::Fn as LiveFields, State::Struct::AphroditeState};

/// Live prefetch schedule - the extended status struct behind the
/// `aphrodite_prefetch_status` tool (pre-Rust `_hooks/prefetch.py`
/// `_prefetch_status_handler` port).
///
/// When `state.live_container` is off the output is byte-identical to the
/// pre-port bridge renderer (aphrodite-hermes `Source/Tool/mod.rs`): `loading` /
/// `ready` / `errors` arrays plus `total_ready`, ready entries carrying exactly
/// `path` / `hash` / `type` / `size`. When on, each ready entry additionally
/// carries the live-container arrival surface (`live`, `arrived_at`,
/// `bytes_arrived`, `eta_s`) and the top level gains `live_container: true`; the
/// base fields are untouched.
///
/// An entry is ready when it carries a `path` in its meta - that is the marker
/// prefetch wrote, and its absence means the marker came from some other
/// transform. The live fields are an optional overlay rather than a second entry
/// shape, so flipping the flag cannot change which entries appear.
pub fn Fn(State:&AphroditeState) -> serde_json::Value {
	let Ready:Vec<serde_json::Value> = State
		.RecentMarkers
		.iter()
		.filter_map(|Marker| {
			let Meta = Marker.meta.as_ref()?;
			let path = Meta.get("path")?;
			let mut Entry = serde_json::json!({
				"path": path,
				"hash": &Marker.hash,
				"type": Marker.CCRType,
				"size": Marker.Size,
			});

			State
				.LiveContainer
				.then(|| LiveFields(Meta))
				.into_iter()
				.flatten()
				.for_each(|(Key, Value)| {
					Entry[Key] = Value;
				});

			Some(Entry)
		})
		.collect();

	let mut Out = serde_json::json!({
		"loading": [],
		"ready": Ready,
		"errors": [],
		"total_ready": Ready.len(),
	});

	// The flag is the value the overlay exists as: when it is off there is no
	// field to set, and the base document stays byte-identical.
	State
		.LiveContainer
		.then_some(serde_json::json!(true))
		.into_iter()
		.for_each(|Live| {
			Out["live_container"] = Live;
		});

	Out
}
