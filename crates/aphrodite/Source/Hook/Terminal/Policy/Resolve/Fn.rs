//! Resolve the terminal delivery policy from the session.

use crate::{
	Hook::{AutoExpand::Applies::Fn::Fn as Applies, Terminal::Policy::Struct::Struct},
	State::Struct::AphroditeState,
};

/// Snapshot every knob a terminal transform decides from.
///
/// One read of the session per transform, before any gate runs, so the gates
/// themselves are pure. The auto-expand predicate is evaluated exactly once
/// here; it used to be evaluated at threshold gating, at live-container
/// selection and again at expansion, and three evaluations of one predicate are
/// three chances for them to disagree.
pub(crate) fn Fn(State:&AphroditeState, Content:&str) -> Struct {
	Struct {
		AutoExpand:Applies(State, Content),
		Threshold:State.Configuration.TerminalThreshold,
		Live:State.Configuration.LiveContainer,
		ChainSplit:State.ChainSplit.Enable,
	}
}
