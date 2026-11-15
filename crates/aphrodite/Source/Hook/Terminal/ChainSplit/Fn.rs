//! Split marked terminal output into per-segment markers.

use crate::{
	ChainSplit::{
		Const::SegMarker,
		ErrorHint::Fn::Fn as SegmentErrorHint,
		Marked::Output::Fn::Fn as SplitMarkedOutput,
	},
	Marker::Format::Entry::CCRMarker,
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Split marked output into one stored, previewed, retrievable marker per
/// segment. Returns `None` when the feature is off, when auto-expand owns the
/// content, when the content carries no segment marker, or when splitting
/// produced fewer than two parts - in each of those cases the caller falls
/// through to the ordinary gates and nothing is split.
///
/// Invisibility contract: the JSON payload carries the per-segment
/// markers array (`Markers`) - the Hermes bridge (replacement_from in
/// aphrodite-hermes) joins those markers into what the LLM sees, so it
/// reads exactly like any compressed output (N natural `<<<CCR:...>>>`
/// markers, each retrievable by its own hash). The `Summary` string is
/// telemetry/metrics ONLY and must never be returned to the LLM - it
/// announces the mechanism.
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, DoAutoExpand:bool) -> Option<serde_json::Value> {
	if !(State.ChainSplitEnabled && !DoAutoExpand && Content.contains(SegMarker)) {
		return None;
	}
	let Parts = SplitMarkedOutput(Content);
	if Parts.len() < 2 {
		return None;
	}
	let mut Segments = Vec::with_capacity(Parts.len());
	let mut TotalOrig = 0usize;
	let mut TotalMarker = 0usize;
	let mut SegHashes = Vec::with_capacity(Parts.len());
	for (Index, SegText) in &Parts {
		TotalOrig += SegText.len();
		let Ct = headroom_core::transforms::content_detector::detect_content_type(SegText).content_type;
		let TypeStr = Ct.as_str().to_string();
		let SegHash = headroom_core::ccr::compute_key(SegText.as_bytes());
		State.InlineStorePut(SegHash.clone(), SegText.to_string());
		SegHashes.push(SegHash.clone());
		let mut Preview = crate::Preview::Builder::Entry::Fn(&TypeStr, SegText);
		// Tier 3: per-segment error hint - content-level only. When
		// this segment's own output carries an error signal, surface
		// it in the preview so the LLM sees which segment failed and
		// why, without retrieving. Never mechanism vocabulary.
		// build-type previews already carry the first `error[...]`
		// message, so skip the hint there (no duplication). The hint
		// is merged INSIDE the preview's brackets: `[text:2L 63B |
		// scanning... ⚠ grep: ...]`, never appended after the `]`.
		if !Preview.contains("error[")
			&& let Some(Hint) = SegmentErrorHint(SegText)
		{
			let Inner = Preview.trim_end_matches(']');
			Preview = format!("{} ⚠ {}]", Inner, Hint);
		}
		let Marker = CCRMarker(&SegHash, &TypeStr, SegText.len(), &Preview, None, None, None);
		TotalMarker += Marker.len();
		State.RecordMarker(MarkerEntry {
			hash:SegHash.clone(),
			CCRType:TypeStr.clone(),
			Size:SegText.len(),
			Preview:Preview.clone(),
			Turn:State.TurnCounter,
			Center:None,
			meta:None,
		});
		Segments.push(serde_json::json!({
			"index": Index,
			"type": TypeStr,
			"size": SegText.len(),
			"hash": SegHash,
			"preview": Preview,
			"marker": Marker,
		}));
	}
	// Tier 1 teaching loop: record the split event so segment
	// retrievals can be attributed and the split threshold adapted.
	State.RecordChainSplit(SegHashes);
	let Summary = format!("[chain:{} segs | {} orig → {} markers]", Segments.len(), TotalOrig, TotalMarker);
	// DEV-ONLY debug metrics (crate::debug): count the chain-split
	// compress decision (per-segment markers are counted individually
	// by `RecordMarker`).
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Compressions);
	Some(serde_json::json!({
			"status": "ok",
			"compressed": true,
			"chain_split": true,
		"segments": Segments.len(),
		"summary": Summary,
		"markers": Segments,
	}))
}
