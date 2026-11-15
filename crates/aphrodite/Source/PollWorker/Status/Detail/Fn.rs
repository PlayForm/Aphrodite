//! The lines one active task contributes to the status block.
//!
//! Two lines at most: the task line always, the live-container line only while the
//! feature is on. Keeping the live line here rather than at the renderer is what
//! makes "off-path is byte-identical" a property of this function instead of a
//! promise the caller has to keep.

use crate::{PollWorker::Task::Struct::BgTask, State::Struct::AphroditeState};

/// The task line, plus the live-container line when the feature is enabled.
pub(crate) fn Fn(State:&AphroditeState, Task:&BgTask) -> Vec<String> {
	let Polled = match Task.PollCount {
		0 => String::from("not polled yet"),
		Count => format!("polled {Count}×"),
	};

	let Line = format!("  · `{}` - running since turn {} ({})", Task.command, Task.StartedTurn, Polled);

	// Live-container progress (pre-Rust `_hooks/live.py` port): ETA is the
	// elapsed-so-far heuristic (0.0 on a just-created task = no time elapsed yet,
	// not "arrived").
	let Live = State.LiveContainer.then(|| {
		format!(
			"    · [live container] {} bytes arrived so far, ETA ~{:.1}s",
			Task.LiveBytesArrived, Task.LiveEtaS
		)
	});

	std::iter::once(Line).chain(Live).collect()
}
