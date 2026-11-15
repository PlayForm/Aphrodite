//! Compute `WindowStats` over the last `n` turns.
//!
//! The window is open at the floor: an event counts while `turn >
//! turn_counter - n`. Off-by-one here changes every telemetry number
//! downstream, and the numbers are read as behaviour ("the model is
//! re-reading files") rather than as diagnostics, so the boundary has to be
//! exactly what the caller means.

use crate::{Flow::Window::Stats::Struct::WindowStats, State::Struct::AphroditeState};

/// Compute `WindowStats` over the last `n` turns (P2/T7).
pub fn TurnWindow(State:&AphroditeState, n:usize) -> WindowStats {
	let Floor = State.TurnCounter.saturating_sub(n);
	let mut Stats = WindowStats::default();
	let mut ErrorSigs = std::collections::HashSet::new();
	let mut SeenPaths = std::collections::HashSet::new();
	for Event in State.ToolEvents.iter().filter(|e| e.Turn > Floor) {
		Stats.TotalCalls += 1;
		if !Event.Okay {
			Stats.Errors += 1;
			if let Some(Sig) = Event.ErrorSig {
				ErrorSigs.insert(Sig);
			}
		}
		if let Some(path) = &Event.WrotePath {
			Stats.Writes += 1;
			if SeenPaths.insert(path.clone()) {
				Stats.NewFiles += 1;
			}
		} else {
			match Event.Tool.as_str() {
				"read_file" => Stats.Reads += 1,
				"search_files" => Stats.Searches += 1,
				_ => {},
			}
		}
	}
	Stats.DistinctErrorSigs = ErrorSigs.len();
	Stats
}
