//! Build the machine-consumed catalog JSON.

use std::collections::BTreeMap;

use crate::Catalog::Size::Fn::Fn as FmtSize;
use crate::State::Struct::AphroditeState;
use crate::StructExtract::Format::Floor::Fn as FloorBoundary;

/// Full catalog result with items, by_type grouping, stats.
pub fn Fn(State:&AphroditeState, Mode:&str) -> serde_json::Value {
	let Items:Vec<serde_json::Value> = State
		.RecentMarkers
		.iter()
		.map(|m| {
			if Mode == "toc" {
				serde_json::json!({
					// Full 40-char hash: a truncated one is unresolvable via
					// exact-match retrieval (F3). Truncation is reserved for the
					// human-readable markdown table (format_catalog_table), not
					// machine-consumed JSON.
					"hash": &m.hash,
					"type": m.CCRType,
					"size": m.Size,
					"preview": FloorBoundary(&m.Preview, 120),
				})
			} else {
				serde_json::json!({
					"hash": m.hash,
					"type": m.CCRType,
					"size": m.Size,
					"preview": FloorBoundary(&m.Preview, 120),
					"turn": m.Turn,
				})
			}
		})
		.collect();

	// Group by type. BTreeMap (not HashMap) so the JSON keys are emitted in
	// sorted order: this workspace builds serde_json with `preserve_order`,
	// so a HashMap's random iteration order leaks into the serialized
	// `ByType` object and identical states produce different JSON across
	// calls.
	let mut ByType:BTreeMap<String, Vec<String>> = BTreeMap::new();
	for m in &State.RecentMarkers {
		ByType.entry(m.CCRType.clone()).or_default().push(m.hash.clone());
	}

	let ByTypeJSON:serde_json::Map<String, serde_json::Value> = ByType
		.iter()
		.map(|(t, Hashes)| {
			(
				t.clone(),
				serde_json::json!({
					"count": Hashes.len(),
					"hashes": &Hashes[..10.min(Hashes.len())],
				}),
			)
		})
		.collect();

	// Savings per entry, not raw original size (report 05 F5): the marker
	// that replaces the content is what the agent actually receives, so
	// "saved" must subtract what still has to be sent. `MarkerEntry` doesn't
	// carry the exact rendered marker length, so the preview length (plus
	// its small fixed template overhead) is the best available stand-in -
	// this is still far closer to reality than reporting the full original
	// size as "saved", which is what a 100%-compression ratio would claim.
	let TotalSaved:usize = State.RecentMarkers.iter().map(|m| m.Size.saturating_sub(m.Preview.len())).sum();

	let mut Result = serde_json::json!({
		"total": Items.len(),           // backward compat with inline callers
		"total_items": Items.len(),
		"total_saved": TotalSaved,
		"total_saved_human": FmtSize(TotalSaved),
		"by_type": ByTypeJSON,
		"items": Items,
		"turn": State.TurnCounter,     // backward compat with inline callers
		"conv_turns": State.ConvIndex.len(),
		"referenced_files": State.ReferencedFiles.len(),
	});

	// TOC mode adds retrieve recommendations
	if Mode == "toc" && !Items.is_empty() {
		let Recommendations:Vec<String> = State
			.RecentMarkers
			.iter()
			.take(10)
			.map(|m| format!("{} ({}B) - {}", m.CCRType, m.Size, FloorBoundary(&m.Preview, 60)))
			.collect();
		Result["recommendations"] = serde_json::json!(Recommendations);
		// `[templates.prompts] search_hint` (pre-Rust port); falls back to
		// the compiled-in string when the key is absent.
		Result["hint"] = serde_json::json!(crate::Flow::Prompt::Render::Fn::RenderPrompt(State, "search_hint", &[]));
	}

	Result
}
