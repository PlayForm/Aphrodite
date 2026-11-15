//! Apply a prepared chain batch to the session.

use crate::{
	ChainSplit::Prepared::Struct::Struct as Prepared,
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Store every prepared segment, record its marker, and render the payload.
///
/// This is the only phase that mutates state, and it is not a transaction: each
/// segment is stored and recorded as it is reached, so a panic part-way through
/// leaves the earlier segments stored and recorded and the later ones absent.
/// That is acceptable because the payload is built in the same pass and is only
/// returned on completion - a half-applied batch never reaches the model - but
/// it is stated rather than implied, so nobody reads the prepare/apply split as
/// an atomicity guarantee it does not provide.
///
/// The payload shape is the invisibility contract: `Markers` carries one marker
/// per segment and the bridge joins them, so the model reads N ordinary
/// `<<<CCR:...>>>` markers and never learns a split happened. `Summary` is
/// telemetry only and must not be returned to the model - it announces the
/// mechanism.
pub(crate) fn Fn(State:&mut AphroditeState, Prepared:&Prepared) -> serde_json::Value {
	let Markers:Vec<serde_json::Value> = Prepared
		.Segments
		.iter()
		.map(|Segment| {
			State.InlineStorePut(Segment.Hash.clone(), Segment.Text.clone());
			State.RecordMarker(MarkerEntry {
				hash:Segment.Hash.clone(),
				CCRType:Segment.Type.clone(),
				Size:Segment.Text.len(),
				Preview:Segment.Preview.clone(),
				Turn:State.TurnCounter,
				Center:None,
				meta:None,
			});

			serde_json::json!({
				"index": Segment.Index,
				"type": Segment.Type,
				"size": Segment.Text.len(),
				"hash": Segment.Hash,
				"preview": Segment.Preview,
				"marker": Segment.Marker,
			})
		})
		.collect();

	// Tier 1 teaching loop: record the split event so segment retrievals can be
	// attributed and the split threshold adapted.
	State.RecordChainSplit(Prepared.Segments.iter().map(|Segment| Segment.Hash.clone()).collect());

	let Summary = format!(
		"[chain:{} segs | {} orig → {} markers]",
		Markers.len(),
		Prepared.OriginalBytes,
		Prepared.MarkerBytes
	);

	// DEV-ONLY debug metrics: the per-segment markers are counted individually
	// by `RecordMarker`; this counts the split decision itself.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Compressions);

	serde_json::json!({
		"status": "ok",
		"compressed": true,
		"chain_split": true,
		"segments": Markers.len(),
		"summary": Summary,
		"markers": Markers,
	})
}
