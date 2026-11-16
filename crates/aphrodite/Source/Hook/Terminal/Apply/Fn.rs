//! Execute a selected terminal plan.

use crate::{
	ChainSplit::Prepared::Apply::Fn::Fn as ApplyPrepared,
	Hook::{
		Classification::{Fn::Fn as Classification, Hint::Enum::Enum as Hint},
		Emit::Fn::Fn as Emit,
		Live::Container::Fn::Fn as Container,
		Terminal::Plan::Enum::Enum as Plan,
	},
	State::Struct::AphroditeState,
};

/// The narrow interpreter: one arm per plan, no policy re-tested.
///
/// Every decision was taken during selection, so this leaf only performs the
/// effects the chosen plan names. It receives no input and no policy of its own:
/// each plan carries the content it transforms and the one execution option it
/// needs, so a plan selected for one content value cannot be executed against
/// another.
///
/// The classification hint is the path's own identity (`Hint::Terminal`), which
/// is what lets shell-shaped output be typed `terminal` here and only here.
pub(crate) fn Fn(State:&mut AphroditeState, Plan:Plan<'_>) -> serde_json::Value {
	match Plan {
		Plan::Passthrough(Reason) => crate::Hook::Passthrough::Fn::Fn(Reason.Text()),
		Plan::Split(Prepared) => ApplyPrepared(State, Prepared),
		Plan::Live { Input } => {
			let Resolved = Classification(Input.Content, Input.Classify, Hint::Terminal);

			Container(State, Input.Content, &Resolved.Type)
		},
		Plan::Compress { Input, AutoExpand } => Emit(State, Input.Content, Input.Classify, Hint::Terminal, AutoExpand),
	}
}
