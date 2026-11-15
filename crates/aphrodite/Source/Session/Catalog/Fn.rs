//! The per-turn catalog summary - delta-only emission for markers AND files.
//!
//! Emitted exactly once per turn by the Flow assembler. Both deltas advance
//! their watermark only after the whole summary string is assembled, so a
//! mid-build failure cannot desync the bookkeeping from what the model
//! actually read; calling it twice in one turn therefore reports "no change"
//! for both.

use crate::State::Struct::AphroditeState;

/// Per-turn catalog summary - delta-only emission for markers AND files (04-F1/F4).
///
/// Markers: +N new compressions this turn, or "no change" cache-stable line.
/// Files: +N new files this turn, or "no change" cache-stable line.
/// Archived turns: count only (changes slowly, ~40 chars, acceptable).
///
/// Once-per-turn contract: `Flow::build_turn_context` is the only production
/// caller and invokes this exactly once per turn. The `last_emitted_*`
/// watermarks are committed only after the summary string is fully
/// assembled, so a failure while building the string can never desync the
/// delta bookkeeping from what was actually emitted; calling this again
/// within the same turn therefore reports "no change" for both deltas.
pub fn CatalogSummary(State:&mut AphroditeState) -> String {
	let CurrentMarkers = State.RecentMarkers.len();
	let PreviousMarkers = State.LastEmittedMarkerCount;

	let mut Parts = vec![];

	// ── Delta markers ──
	let NewMarkers = CurrentMarkers.saturating_sub(PreviousMarkers);
	if NewMarkers > 0 && CurrentMarkers > 0 {
		let Previews:Vec<String> = State
			.RecentMarkers
			.iter()
			.rev()
			.take(NewMarkers)
			.map(|m| m.Preview.clone())
			.collect();
		Parts.push(format!(
			"[Aphrodite] +{} new compression{} this turn: {}. {} total.",
			NewMarkers,
			if NewMarkers == 1 { "" } else { "s" },
			Previews.join(", "),
			CurrentMarkers,
		));
	} else if CurrentMarkers > 0 {
		Parts.push(format!("[Aphrodite] {} compressions (no change this turn).", CurrentMarkers));
	}

	// ── Archived turns ──
	let ConvCount = State.ConvIndex.len();
	if ConvCount > 0 {
		Parts.push(format!("{} archived turns available for retrieval.", ConvCount));
	}

	// ── Delta files (04-F4) ──
	let CurrentFiles = State.ReferencedFiles.len();
	let PreviousFiles = State.LastEmittedFileCount;

	if CurrentFiles > 0 {
		let NewFiles = CurrentFiles.saturating_sub(PreviousFiles);
		if NewFiles > 0 {
			// `RecordFile` pushes new entries to the FRONT of the deque,
			// so the new ones sit at the front; take from the front, then
			// reverse so names render in insertion order (oldest-of-new
			// first). Reading from the back would select the OLDEST entries
			// and misreport old files as new.
			let Names:Vec<String> = State
				.ReferencedFiles
				.iter()
				.take(NewFiles)
				.map(|(p, t)| format!("{} ({})", p, t))
				.collect::<Vec<_>>()
				.into_iter()
				.rev()
				.collect();
			Parts.push(format!(
				"+{} new file{}: {}. {} total.",
				NewFiles,
				if NewFiles == 1 { "" } else { "s" },
				Names.join(", "),
				CurrentFiles
			));
		} else {
			Parts.push(format!("{} referenced files (no change).", CurrentFiles));
		}
	}

	// ── Poll workers ──
	if State.PollWorkerEnabled {
		let Active:Vec<_> = State
			.BgTasks
			.iter()
			.filter(|t| t.Status == crate::PollWorker::Task::Status::Enum::BgStatus::Running)
			.collect();
		if !Active.is_empty() {
			Parts.push(format!("{} active poll worker(s). Use process(action='poll').", Active.len()));
		}
	}

	// ── Intent hints (prompt knob `[prompts] catalog_intent_hints`, opt-in) ──
	// Pre-Rust port: `intent=read | recent CCRs: <last 3 hashes, 12 chars>`.
	// Default off = current behavior.
	if State.CatalogIntentHints && CurrentMarkers > 0 {
		let Last3:Vec<_> = State.RecentMarkers.iter().rev().take(3).collect();
		let Hashes:Vec<String> = Last3.iter().rev().map(|m| m.hash.chars().take(12).collect()).collect();
		Parts.push(format!("intent=read | recent CCRs: {}", Hashes.join(" ")));
	}

	let Summary = Parts.join(" ");

	// Commit both watermarks only now, after the string is fully assembled:
	// the two counters advance together, and a mid-build failure can't leave
	// one side of the delta bookkeeping ahead of the other (Fix 4).
	State.LastEmittedMarkerCount = CurrentMarkers;
	State.LastEmittedFileCount = CurrentFiles;

	Summary
}
