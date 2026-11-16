//! The lines one active task contributes to the status block.
//!
//! Two lines at most: the task line always, the live-container line only while the
//! feature is on. Keeping the live line here rather than at the renderer is what
//! makes "off-path is byte-identical" a property of this function instead of a
//! promise the caller has to keep.
//!
//! The flag arrives as a `bool`, not as the session: this renderer reads one setting
//! and one task, and taking the state would let it grow a dependency on anything else.

use crate::PollWorker::Task::Struct::Struct as Record;

/// The task line, plus the live-container line when the feature is enabled.
pub(crate) fn Fn(Live:bool, Task:&Record) -> Vec<String> {
	let Polled = match Task.PollCount {
		0 => String::from("not polled yet"),
		Count => format!("polled {Count}×"),
	};

	let Line = format!("  · `{}` - running since turn {} ({})", Task.command, Task.StartedTurn, Polled);

	// Live-container progress (pre-Rust `_hooks/live.py` port): ETA is the
	// elapsed-so-far heuristic (0.0 on a just-created task = no time elapsed yet,
	// not "arrived").
	let LiveLine = Live.then(|| {
		format!(
			"    · [live container] {} bytes arrived so far, ETA ~{:.1}s",
			Task.LiveBytesArrived, Task.LiveEtaS
		)
	});

	std::iter::once(Line).chain(LiveLine).collect()
}
