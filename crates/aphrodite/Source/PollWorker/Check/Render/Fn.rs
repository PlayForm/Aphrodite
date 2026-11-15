//! Render one task's nudge.
//!
//! The strings are behaviour: they are what the model reads, and every clause in
//! them earns its place - the command so the agent knows which build, the exit
//! code so it knows whether to retrieve or investigate, the hash so the retrieve
//! call is one step, the preview so it can decide whether to retrieve at all.
//! `None` for a stale task: `Select` never hands one over, and this function
//! refuses rather than inventing a sentence for a case that has none.

use crate::PollWorker::{
	Check::{Plan::Struct::NudgePlan, Polled::Fn::Fn as Polled},
	Task::{Status::Enum::BgStatus, Struct::BgTask},
};

/// The nudge for this task, or `None` when it has nothing to say.
pub(crate) fn Fn(Task:&BgTask) -> Option<NudgePlan> {
	let Plan = match &Task.Status {
		BgStatus::Running => {
			NudgePlan {
				Text:format!(
					"poll worker: `{}` still running since turn {}{}. Use process(action='poll').",
					Task.command,
					Task.StartedTurn,
					Polled(Task)
				),
				TtlTurns:1,
			}
		},
		BgStatus::Done { ExitCode } => {
			NudgePlan {
				Text:format!(
					"✓ poll worker: `{}` completed (exit {}). Retrieve: aphrodite_retrieve(hash={}) - {}",
					Task.command,
					ExitCode,
					Task.OutputHash.as_deref().unwrap_or("?"),
					Task.OutputPreview.as_deref().unwrap_or("[output available]")
				),
				// Survive 2 turns so the agent has time to act.
				TtlTurns:2,
			}
		},
		BgStatus::Failed { ExitCode } => {
			NudgePlan {
				Text:format!(
					"✗ poll worker: `{}` failed (exit {}). Retrieve: aphrodite_retrieve(hash={}) - {}",
					Task.command,
					ExitCode,
					Task.OutputHash.as_deref().unwrap_or("?"),
					Task.OutputPreview.as_deref().unwrap_or("[error output available]")
				),
				TtlTurns:2,
			}
		},
		// Don't nudge about stale tasks - they're abandoned.
		BgStatus::Stale => return None,
	};

	Some(Plan)
}
