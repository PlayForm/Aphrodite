//! The `PreLLMCall` nudge pass: tell the agent which tasks need it.
//!
//! Bounded on purpose. Every nudge spends part of the per-turn directive
//! budget, so the pass emits at most two and walks newest-first; a third
//! running build would otherwise crowd out the behavioural directives the
//! turn actually needs. Stale tasks are silent - the agent already left them.

use crate::{PollWorker::Task::Status::Enum::BgStatus, State::Struct::AphroditeState};

/// Check all background tasks and push ephemeral nudges for any that
/// need the agent's attention. Called from `PreLLMCall`.
///
/// Pushes at most 2 nudges (newest-first for active tasks) so the
/// directive/nudge budget isn't consumed by poll workers.
pub fn CheckBgTasks(State:&mut AphroditeState) {
	let mut Nudged = 0usize;

	// Collect indices to nudge (iterate in insertion order = oldest first,
	// nudging newest active tasks within the 2-nudge cap).
	let TaskCount = State.BgTasks.len();
	let _ = (0..TaskCount).rev().try_for_each(|i| -> Result<(), ()> {
		match Nudged >= 2 {
			true => {
				return Err(());
			},
			false => {},
		}
		let task = &State.BgTasks[i];
		match &task.Status {
			BgStatus::Running => {
				let Last = match task.PollCount > 0 {
					true => format!(" (polled {}×, last turn {})", task.PollCount, task.LastPollTurn),
					false => String::from(" (not polled yet)"),
				};
				crate::Flow::Nudge::Push::Fn::PushNudge(
					State,
					&format!(
						"poll worker: `{}` still running since turn {}{}. Use process(action='poll').",
						task.command, task.StartedTurn, Last
					),
					1,
				);
				Nudged += 1;
			},
			BgStatus::Done { ExitCode } => {
				let HashPreview = task.OutputPreview.as_deref().unwrap_or("[output available]");
				crate::Flow::Nudge::Push::Fn::PushNudge(
					State,
					&format!(
						"✓ poll worker: `{}` completed (exit {}). Retrieve: aphrodite_retrieve(hash={}) - {}",
						task.command,
						ExitCode,
						task.OutputHash.as_deref().unwrap_or("?"),
						HashPreview,
					),
					2, // survive 2 turns so the agent has time to act
				);
				Nudged += 1;
			},
			BgStatus::Failed { ExitCode } => {
				let HashPreview = task.OutputPreview.as_deref().unwrap_or("[error output available]");
				crate::Flow::Nudge::Push::Fn::PushNudge(
					State,
					&format!(
						"✗ poll worker: `{}` failed (exit {}). Retrieve: aphrodite_retrieve(hash={}) - {}",
						task.command,
						ExitCode,
						task.OutputHash.as_deref().unwrap_or("?"),
						HashPreview,
					),
					2,
				);
				Nudged += 1;
			},
			BgStatus::Stale => {
				// Don't nudge about stale tasks - they're abandoned.
			},
		}
		Ok(())
	});
}
