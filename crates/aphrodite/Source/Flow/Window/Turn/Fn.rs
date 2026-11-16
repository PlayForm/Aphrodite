//! Compute the window aggregate over the last `n` turns.
//!
//! The window is open at the floor: an event counts while `turn >
//! turn_counter - n`. Off-by-one here changes every telemetry number
//! downstream, and the numbers are read as behaviour ("the model is
//! re-reading files") rather than as diagnostics, so the boundary has to be
//! exactly what the caller means.

use crate::{Flow::Window::Stats::Struct::Struct as Window, State::Struct::AphroditeState};

/// Compute `Window::Stats::Struct` over the last `n` turns (P2/T7).
pub fn Fn(State:&AphroditeState, n:usize) -> Window {
	let Floor = State.Turn.Counter.saturating_sub(n);
	let mut Stats = Window::default();
	let mut ErrorSigs = std::collections::HashSet::new();
	let mut SeenPaths = std::collections::HashSet::new();
	State.Event.Recent.iter().filter(|e| e.Turn > Floor).for_each(|Event| {
		Stats.Total += 1;
		// A failed call is one count and (maybe) one signature; a successful one
		// contributes neither, so the boolean is the quantity rather than a gate.
		Stats.Errors += (!Event.Okay) as usize;

		// The set decides what it holds: `None` from a successful call or from an
		// error without a signature extends it with nothing.
		ErrorSigs.extend((!Event.Okay).then_some(Event.ErrorSig).flatten());

		match &Event.WrotePath {
			Some(path) => {
				Stats.Writes += 1;
				// `insert` reports whether the path was new, which is exactly the
				// increment - the set decides, no second lookup.
				Stats.NewFiles += SeenPaths.insert(path.clone()) as usize;
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

	Stats.Distinct = ErrorSigs.len();

	Stats
}
