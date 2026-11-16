//! The two entry points that decide whether a tool call becomes a poll worker.

use crate::{
	PollWorker::Should::{Command::Fn::Fn as Match, Floor::Fn::Fn as Floor, Plan::Struct::Struct as Background},
	State::Identity::Command::Struct::Struct as Command,
};

/// Post-execution predicate: the output is in hand, so the size floor applies too.
///
/// `Some((identity, label))` when the command is known-slow and the output large
/// enough to be worth tracking, `None` to let it pass through. The identity is the
/// command identity the poll worker later deduplicates on, and the label is what the
/// marker and the nudges show.
///
/// The caller (agent bridge) is responsible for deciding *which* tools to check -
/// this function only evaluates the command string and content size.
pub fn After(Content:&str, command:Option<&str>) -> Option<(Command, String)> {
	let Plan = Floor(Content).then(|| Match(command)).flatten()?;

	Some(Pair(Plan))
}

/// Pre-execution predicate: only the command is known (the tool has not run yet).
///
/// Used by the `pre_tool_call` hook to decide whether to auto-inject
/// `background=true`; the shape is the same pair `After` answers, so the two hooks
/// cannot disagree about which command is backgroundable.
pub fn Before(command:Option<&str>) -> Option<(Command, String)> { Match(command).map(Pair) }

/// The public pair shape, projected from the plan.
///
/// A tuple rather than the plan itself because this is the crate's public surface for
/// the bridge, and the plan's label is the only field it reads. The identity half stays
/// typed, so a caller cannot substitute a different kind of identity for it.
fn Pair(Plan:Background) -> (Command, String) { (Plan.Identity, Plan.Label) }
