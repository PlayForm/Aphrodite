//! The gates that follow the split attempt.

use crate::Hook::{
	Reason::Enum::Enum as Reason,
	Terminal::{Plan::Enum::Enum as Plan, Policy::Struct::Struct as Policy},
};

/// The unmarked-output threshold, then the live container, then compression.
///
/// Third and fourth in the contract's order, and both are refused under
/// auto-expand: auto-expand promises no marker reaches the model, so neither a
/// threshold passthrough nor a container marker may take the content first.
///
/// The threshold applies to UNMARKED output - content that reached this point
/// either carried no segment marker or could not be split, so there is no marker
/// in it to leak. A threshold of `0` means "always compress".
pub(crate) fn Fn(Policy:&Policy, Content:&str) -> Plan {
	let Unmarked = !Policy.AutoExpand;
	let BelowThreshold = Unmarked && Policy.Threshold > 0 && Content.len() < Policy.Threshold;
	let Live = Policy.Live && Unmarked;

	[(BelowThreshold, Plan::Passthrough(Reason::BelowThreshold)), (Live, Plan::Live)]
		.into_iter()
		.find(|(Hit, _)| *Hit)
		.map(|(_, Plan)| Plan)
		.unwrap_or(Plan::Compress)
}
