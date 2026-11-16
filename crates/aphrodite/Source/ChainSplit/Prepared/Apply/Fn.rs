//! Apply a prepared chain batch to the session.

use crate::{
	ChainSplit::Prepared::{Claims::Fn::Fn as BatchClaims, Struct::Struct as Prepared},
	State::{Marker::Entry::Struct::Struct as MarkerEntry, Struct::AphroditeState},
};

/// Store every prepared segment, record its marker, and render the payload.
///
/// The batch is CONSUMED: a prepared batch is applied at most once, because
/// applying it twice would store the same segments twice and record two markers
/// for one split. Taking it by value is what makes the second application
/// unrepresentable rather than merely unwise, and it lets each segment's text
/// move into the store instead of being cloned into it - the text is the one
/// value in the batch that has exactly one owner at this point.
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
pub(crate) fn Fn(State:&mut AphroditeState, Prepared:Prepared) -> serde_json::Value {
	let OriginalBytes = Prepared.OriginalBytes;
	let MarkerBytes = Prepared.MarkerBytes;
	// The ledger's claims are content hashes, so they are read before the batch
	// is consumed and before any segment is stored: a claim describes what
	// preparation produced, not what the store happened to keep.
	let Claims = BatchClaims(&Prepared);

	let Markers:Vec<serde_json::Value> = Prepared
		.Segments
		.into_iter()
		.map(|Segment| {
			// `Size` is read before the text moves into the store; the marker
			// entry and the payload both describe those same bytes.
			let Size = Segment.Text.len();
			State.Store.Put(Segment.Hash.clone(), Segment.Text);
			State.Marker.Record(MarkerEntry {
				hash:Segment.Hash.clone(),
				CCRType:Segment.Type.clone(),
				Size,
				Preview:Segment.Preview.clone(),
				Turn:State.Turn.Counter,
				Center:None,
				meta:None,
			});

			serde_json::json!({
				"index": Segment.Index,
				"type": Segment.Type,
				"size": Size,
				"hash": Segment.Hash,
				"preview": Segment.Preview,
				"marker": Segment.Marker,
			})
		})
		.collect();

	// Tier 1 teaching loop: record the split event so segment retrievals can be
	// attributed and the split threshold adapted.
	State.ChainSplit.Record(State.Turn.Counter, Claims);

	let Summary = format!(
		"[chain:{} segs | {} orig → {} markers]",
		Markers.len(),
		OriginalBytes,
		MarkerBytes
	);

	// DEV-ONLY debug metrics: the per-segment markers are counted individually
	// by `Marker::Record`; this counts the split decision itself.
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
