//! The per-turn catalog summary - delta-only emission for markers AND files.
//!
//! Emitted exactly once per turn by the Flow assembler. Both deltas advance their
//! watermark only after the whole summary string is assembled, so a mid-build
//! failure cannot desync the bookkeeping from what the model actually read;
//! calling it twice in one turn therefore reports "no change" for both.

use crate::{
	Session::Catalog::{
		Archived::Fn::Fn as Archived,
		FileDelta::Fn::Fn as FileDelta,
		Intent::Fn::Fn as Intent,
		MarkerDelta::Fn::Fn as MarkerDelta,
		Workers::Fn::Fn as Workers,
	},
	State::Struct::AphroditeState,
};

/// Per-turn catalog summary - delta-only emission for markers AND files
/// (04-F1/F4).
///
/// Markers: +N new compressions this turn, or a "no change" cache-stable line.
/// Files: +N new files this turn, or a "no change" cache-stable line.
/// Archived turns: count only (changes slowly, ~40 chars, acceptable).
///
/// Once-per-turn contract: `Flow::TurnContext::Fn::BuildTurnContext` is the only
/// production caller and invokes this exactly once per turn. The
/// `last_emitted_*` watermarks are committed only after the summary string is
/// fully assembled, so a failure while building it can never desync the delta
/// bookkeeping from what was actually emitted.
///
/// The five sections are five optional lines in a fixed order, joined by one
/// space. A section that has nothing to say answers `None` rather than an empty
/// string, so "absent" and "empty" are the same fact and the join cannot leave a
/// stray separator.
pub fn CatalogSummary(State:&mut AphroditeState) -> String {
	let CurrentMarkers = State.RecentMarkers.len();
	let PreviousMarkers = State.LastEmittedMarkerCount;
	let CurrentFiles = State.ReferencedFiles.len();
	let PreviousFiles = State.LastEmittedFileCount;
	let NewMarkers = CurrentMarkers.saturating_sub(PreviousMarkers);
	let NewFiles = CurrentFiles.saturating_sub(PreviousFiles);

	let Sections = [
		MarkerDelta(&State.RecentMarkers, NewMarkers, CurrentMarkers),
		Archived(State.ConvIndex.len()),
		FileDelta(&State.ReferencedFiles, NewFiles, CurrentFiles),
		Workers(State.PollWorkerEnabled, &State.BgTasks),
		Intent(State.CatalogIntentHints, &State.RecentMarkers, CurrentMarkers),
	];
	let Summary = Sections.into_iter().flatten().collect::<Vec<String>>().join(" ");

	// Commit both watermarks only now, after the string is fully assembled: the
	// two counters advance together, and a mid-build failure cannot leave one
	// side of the delta bookkeeping ahead of the other.
	State.LastEmittedMarkerCount = CurrentMarkers;
	State.LastEmittedFileCount = CurrentFiles;

	Summary
}
