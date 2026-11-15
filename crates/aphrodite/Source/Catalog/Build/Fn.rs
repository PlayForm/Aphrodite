//! Build the machine-consumed catalog JSON.

use std::collections::BTreeMap;

use super::Enrich;
use crate::{
	Catalog::{Mode, Size::Fn::Fn as FmtSize},
	State::Struct::AphroditeState,
	StructExtract::Format::Floor::Fn as FloorBoundary,
};

/// Full catalog result with items, by_type grouping and stats.
///
/// The mode is parsed ONCE at the boundary and matched from there, so the two
/// projections are a structural choice rather than three repeated `== "toc"` tests -
/// the third of which (the recommendations) also had to restate the emptiness check.
///
/// The JSON keys and the markdown columns are a wire contract: the `AphroditeCatalog`
/// tool and the C ABI return this object, so a renamed key is a breaking change for a
/// caller outside this crate.
pub fn Fn(State:&AphroditeState, Mode:&str) -> serde_json::Value {
	let Mode = Mode::Parse::Fn::Fn(Mode);

	let Items:Vec<serde_json::Value> = State
		.RecentMarkers
		.iter()
		.map(|Marker| {
			match Mode {
				// Full 40-char hash: a truncated one is unresolvable via exact-match
				// retrieval (F3). Truncation is reserved for the human-readable markdown
				// table, not for machine-consumed JSON.
				Mode::Enum::Enum::TOC => {
					serde_json::json!({
						"hash": &Marker.hash,
						"type": Marker.CCRType,
						"size": Marker.Size,
						"preview": FloorBoundary(&Marker.Preview, 120),
					})
				},
				Mode::Enum::Enum::Full => {
					serde_json::json!({
						"hash": Marker.hash,
						"type": Marker.CCRType,
						"size": Marker.Size,
						"preview": FloorBoundary(&Marker.Preview, 120),
						"turn": Marker.Turn,
					})
				},
			}
		})
		.collect();

	// Group by type. BTreeMap (not HashMap) so the JSON keys are emitted in sorted
	// order: this workspace builds serde_json with `preserve_order`, so a HashMap's
	// random iteration order leaks into the serialized `ByType` object and identical
	// states produce different JSON across calls.
	let mut ByType:BTreeMap<String, Vec<String>> = BTreeMap::new();

	State.RecentMarkers.iter().for_each(|Marker| {
		ByType.entry(Marker.CCRType.clone()).or_default().push(Marker.hash.clone());
	});

	let ByTypeJSON:serde_json::Map<String, serde_json::Value> = ByType
		.iter()
		.map(|(Type, Hashes)| {
			(
				Type.clone(),
				serde_json::json!({
					"count": Hashes.len(),
					"hashes": &Hashes[..10.min(Hashes.len())],
				}),
			)
		})
		.collect();

	// Savings per entry, not raw original size (report 05 F5): the marker that
	// replaces the content is what the agent actually receives, so "saved" must
	// subtract what still has to be sent. `MarkerEntry` does not carry the exact
	// rendered marker length, so the preview length (plus its small fixed template
	// overhead) is the best available stand-in - still far closer to reality than
	// reporting the full original size as "saved", which is what a 100%-compression
	// ratio would claim.
	let TotalSaved:usize = State
		.RecentMarkers
		.iter()
		.map(|Marker| Marker.Size.saturating_sub(Marker.Preview.len()))
		.sum();

	let mut Result = serde_json::json!({
		"total": Items.len(),           // backward compat with inline callers
		"total_items": Items.len(),
		"total_saved": TotalSaved,
		"total_saved_human": FmtSize(TotalSaved),
		"by_type": ByTypeJSON,
		"items": Items,
		"turn": State.TurnCounter,      // backward compat with inline callers
		"conv_turns": State.ConvIndex.len(),
		"referenced_files": State.ReferencedFiles.len(),
	});

	Enrich::Fn::Fn(State, &Mode, &mut Result);

	Result
}
