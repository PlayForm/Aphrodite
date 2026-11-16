//! Apply a prepared context-engine archive.

use crate::{
	Session::{Archive::Fn::Fn as Archive, Engine::Plan::Struct::Struct as Plan},
	State::{Marker::Entry::Struct::Struct as Entry, Struct::AphroditeState},
};

/// Store the packed payload and replace the middle of the index with its marker.
///
/// The effects are in the order the model depends on: the payload is stored
/// before the turns that pointed at it are dropped (a drop first would leave the
/// history briefly unretrievable), the archive entry is recorded before the marker
/// so the index and the marker ring agree, and the telemetry is written last.
///
/// This is not a transaction. A panic part-way through leaves the payload stored
/// and the middle still present, which is recoverable - the marker is only
/// recorded after the index is consistent, so the model never reads a marker for
/// an archive that was not finished.
pub(crate) fn Fn(State:&mut AphroditeState, Plan:&Plan) -> Option<String> {
	State.Store.Put(Plan.Hash.clone(), Plan.Packed.clone());

	Plan.MiddleKeys.iter().for_each(|Turn| {
		State.Turn.Index.remove(Turn);
	});

	let Summary = format!(
		"engine offload: {} middle turns archived (keep {}/{})",
		Plan.MiddleN, Plan.HeadN, Plan.TailN
	);
	Archive(&mut State.Turn, &Plan.Hash, &Summary, Plan.Packed.len());
	State.Marker.Record(Entry {
		hash:Plan.Hash.clone(),
		CCRType:"context".into(),
		Size:Plan.Packed.len(),
		Preview:Plan.Preview.clone(),
		Turn:State.Turn.Counter,
		Center:None,
		meta:None,
	});

	// DEV-ONLY debug instrumentation (crate::debug): archive outcome.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::EngineArchives);
	crate::Debug::Log::At::Fn::Fn(
		State.Configuration.DebugLevel,
		"engine_archive",
		&[
			("hash", Plan.Hash.clone()),
			("middle", Plan.MiddleN.to_string()),
			("head", Plan.HeadN.to_string()),
			("tail", Plan.TailN.to_string()),
			("size", Plan.Packed.len().to_string()),
		],
	);
	tracing::info!(
		hash = Plan.Hash,
		Middle = Plan.MiddleN,
		Size = Plan.Packed.len(),
		"context_engine: archived {} middle turns to CCR:{} ({} bytes)",
		Plan.MiddleN,
		Plan.Hash,
		Plan.Packed.len()
	);

	Some(Plan.Prompt.clone())
}
