//! The two entry points that decide whether a tool call becomes a poll
//! worker.
//!
//! They differ only in what they can see: `ShouldBackground` runs after the
//! tool produced output and can therefore also apply the minimum-size floor,
//! while `ShouldBackgroundPre` runs before execution and only has the
//! command. Both are the same decision - a known-slow command - so neither
//! may grow a second heuristic of its own; a divergence between them makes a
//! command background on one hook and not the other, and the agent ends up
//! polling a task that was never created.

use crate::PollWorker::Should::Command::Fn::ShouldBackgroundByCommand;

/// Heuristic: should this tool call's output be replaced with a poll-worker
/// marker so the agent can check progress asynchronously?
///
/// Returns `Some((task_id, command_summary))` if the command/content suggests
/// it should be backgrounded, or `None` to let it pass through normally.
///
/// The caller (agent bridge) is responsible for deciding *which* tools to
/// check - this function only evaluates the command string and content size.
pub fn ShouldBackground(Content:&str, command:Option<&str>) -> Option<(String, String)> {
	// Short output: pass through normally - the agent doesn't need
	// to poll for a 200-byte `Ls`.
	match Content.len() < 2048 {
		true => {
			return None;
		},
		false => {},
	}
	ShouldBackgroundByCommand(command)
}

/// Pre-execution heuristic: check only the command prefix, no content
/// required (we haven't run the tool yet). Used by the `pre_tool_call`
/// hook to decide whether to auto-inject `background=true`.
pub fn ShouldBackgroundPre(command:Option<&str>) -> Option<(String, String)> { ShouldBackgroundByCommand(command) }
