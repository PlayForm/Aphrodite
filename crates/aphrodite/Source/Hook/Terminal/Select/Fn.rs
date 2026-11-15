//! Select a terminal plan.

use crate::Hook::Terminal::{
	Input::Struct::Struct as Input,
	Plan::Enum::Enum as Plan,
	Policy::Struct::Struct as Policy,
	Select::{AfterSplit::Fn::Fn as AfterSplit, Empty::Fn::Fn as Empty, Split::Fn::Fn as Split},
};

/// Choose the plan from the policy and the input, in the contract's order.
///
/// The order is the contract, not an implementation detail: empty → the
/// auto-expand decision → the chain-split attempt → the unmarked-output
/// threshold → the live-container decision → ordinary compression. Reordering
/// any two changes what the model reads, which is why the sequence is spelled
/// out here as three named steps in that order rather than as a flat table.
///
/// Pure: no `&mut AphroditeState`, no store, no marker. Everything the plan
/// carries was derived from the content and the snapshotted policy.
pub(crate) fn Fn(Policy:&Policy, Input:&Input<'_>) -> Plan {
	Empty(Input.Content)
		.or_else(|| Split(Policy, Input.Content))
		.unwrap_or_else(|| AfterSplit(Policy, Input.Content))
}
