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
	State.ToolEvents.iter().filter(|e| e.Turn > Floor).for_each(|Event| {
		Stats.TotalCalls += 1;

		match !Event.Okay {
			true => {
				Stats.Errors += 1;

				match Event.ErrorSig {
					Some(Sig) => {
						ErrorSigs.insert(Sig);
					},
					None => (),
				}
			},
			false => (),
		}

		match &Event.WrotePath {
			Some(path) => {
				Stats.Writes += 1;

				match SeenPaths.insert(path.clone()) {
					true => {
						Stats.NewFiles += 1;
					},
					false => (),
				}
			},
			None => {
				match Event.Tool.as_str() {
					"read_file" => Stats.Reads += 1,
					"search_files" => Stats.Searches += 1,
					_ => {},
				}
			},
		}
	});

	Stats.DistinctErrorSigs = ErrorSigs.len();

	Stats
}
