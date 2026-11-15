//! The two entry points that decide whether a tool call becomes a poll worker.

use crate::PollWorker::Should::{Command::Fn::Fn as Command, Floor::Fn::Fn as Floor, Plan::Struct::BackgroundPlan};

/// Heuristic: should this tool call's output be replaced with a poll-worker marker
/// so the agent can check progress asynchronously?
///
/// Returns `Some((task_id, command_summary))` when the command is known-slow and
/// the output large enough to be worth tracking, or `None` to let it pass through.
///
/// The caller (agent bridge) is responsible for deciding *which* tools to check -
/// this function only evaluates the command string and content size.
pub fn ShouldBackground(Content:&str, command:Option<&str>) -> Option<(String, String)> {
	let Plan = Floor(Content).then(|| Command(command)).flatten()?;

	Some(Pair(Plan))
}

/// Pre-execution heuristic: check only the command prefix, no content required (we
/// have not run the tool yet). Used by the `pre_tool_call` hook to decide whether
/// to auto-inject `background=true`.
pub fn ShouldBackgroundPre(command:Option<&str>) -> Option<(String, String)> { Command(command).map(Pair) }

/// The public pair shape, projected from the plan.
fn Pair(Plan:BackgroundPlan) -> (String, String) { (Plan.Identity, Plan.Label) }
