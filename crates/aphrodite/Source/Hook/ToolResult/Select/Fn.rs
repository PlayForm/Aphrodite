//! Select a tool-result plan.

use crate::Hook::{
	Reason::Enum::Enum as Reason,
	Tool::Const::EssentialTools,
	ToolResult::{Input::Struct::Struct as Input, Plan::Enum::Enum as Plan, Policy::Struct::Struct as Policy},
};

/// Choose the plan from the policy and the input, in the contract's order.
///
/// The order is the contract, not an implementation detail: empty → essential
/// tool → Aphrodite's own tools → the unmarked threshold → the live container →
/// ordinary compression. It is expressed as one ordered table rather than a
/// chain of statements so the precedence is visible as data - the first matching
/// row wins, and the default is the last gate.
///
/// Every row is a pure predicate over the input and the snapshotted policy, and
/// every plan value is fieldless, so building the whole table costs a few moves
/// and cannot skip work that a later row would have needed. No row has an
/// effect, which is what makes it safe to evaluate them all.
///
/// The auto-expand gate is a POLICY, not a row: it does not choose a plan, it
/// removes the threshold and live-container rows, so that under auto-expand the
/// content reaches the compress path (which returns it inline) and no marker can
/// leak.
pub(crate) fn Fn(Policy:&Policy, Input:&Input<'_>) -> Plan {
	let Content = Input.Content;
	let Tool = Input.Tool;
	let Unmarked = !Policy.AutoExpand;
	let Essential = EssentialTools.contains(&Tool);
	let Own = Tool.starts_with("aphrodite_") || Tool.starts_with("headroom");
	let BelowThreshold = Unmarked && Policy.Threshold > 0 && Content.len() < Policy.Threshold;
	let Live = Policy.Live && Unmarked;

	[
		(Content.is_empty(), Plan::Passthrough(Reason::Empty)),
		(Essential, Plan::Passthrough(Reason::EssentialTool)),
		(Own, Plan::Passthrough(Reason::SelfTool)),
		(BelowThreshold, Plan::Passthrough(Reason::BelowThreshold)),
		(Live, Plan::Live),
	]
	.into_iter()
	.find(|(Hit, _)| *Hit)
	.map(|(_, Plan)| Plan)
	.unwrap_or(Plan::Compress)
}
