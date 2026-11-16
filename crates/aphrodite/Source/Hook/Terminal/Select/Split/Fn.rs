//! The chain-split gate.

use crate::{
	ChainSplit::{Prepared::Prepare::Fn::Fn as PrepareChain, Segment::Const::Marker},
	Hook::Terminal::{Input::Struct::Struct as Input, Plan::Enum::Enum as Plan, Policy::Struct::Struct as Policy},
};

/// Split marked output into per-segment markers, when it is a chain.
///
/// Second, before the threshold gate, because below-threshold marked output
/// would otherwise pass through with the segment markers still in it - the
/// model would read `__APHRODITE_SEG__` as if it were output. Splitting marked
/// content is unconditional when the feature is on, except under auto-expand,
/// where the whole content is returned expanded instead and no marker may reach
/// the model at all.
///
/// The prepared batch is built here because preparation is pure and because
/// "this content is a chain" is a fact about the content: a marker split that
/// yields fewer than two parts answers `None`, and the caller falls through to
/// the gates that follow.
pub(crate) fn Fn<'a>(Policy:&Policy, Input:Input<'a>) -> Option<Plan<'a>> {
	(Policy.ChainSplit && !Policy.AutoExpand && Input.Content.contains(Marker))
		.then(|| PrepareChain(Input.Content))
		.flatten()
		.map(Plan::Split)
}
