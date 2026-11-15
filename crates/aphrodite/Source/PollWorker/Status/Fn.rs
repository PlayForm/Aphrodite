//! The turn-context renderer: the poll-worker block the agent reads at the
//! top of a turn.
//!
//! Off-path output is byte-identical to the pre-port renderer - the
//! live-container line appears only when `state.live_container` is on, so
//! enabling the feature can never silently reshape the lines an agent has
//! already been conditioned on. Only running tasks are listed: a finished
//! task is the nudge's job, not the status block's.

use crate::{
	PollWorker::Task::{Status::Enum::BgStatus, Struct::BgTask},
	State::Struct::AphroditeState,
};

/// Render bg task status lines for injection into `BuildTurnContext`.
/// Returns empty string if no active tasks.
pub fn RenderBgTaskStatus(State:&AphroditeState) -> String {
	let Active:Vec<&BgTask> = State.BgTasks.iter().filter(|t| t.Status == BgStatus::Running).collect();

	match Active.is_empty() {
		true => {
			return String::new();
		},
		false => {},
	}

	let mut Lines = vec![format!("[poll workers] {} active task(s):", Active.len())];
	Active.iter().take(3).for_each(|task| {
		let PollInformation = match task.PollCount > 0 {
			true => format!("polled {}×", task.PollCount),
			false => "not polled yet".to_string(),
		};
		Lines.push(format!(
			"  · `{}` - running since turn {} ({})",
			task.command, task.StartedTurn, PollInformation
		));
		// Live-container progress line (pre-Rust `_hooks/live.py` port):
		// only rendered when the feature is enabled, so the off-path stays
		// byte-identical. ETA is the elapsed-so-far heuristic (0.0 on a
		// just-created task = no time elapsed yet, not "arrived").
		match State.LiveContainer {
			true => {
				Lines.push(format!(
					"    · [live container] {} bytes arrived so far, ETA ~{:.1}s",
					task.LiveBytesArrived, task.LiveEtaS
				));
			},
			false => {},
		}
	});
	Lines.join("\n")
}
