//! Execute a selected tool-result plan.

use crate::{
	Hook::{
		Classification::{Fn::Fn as Classification, Hint::Enum::Enum as Hint},
		Emit::Fn::Fn as Emit,
		Live::Container::Fn::Fn as Container,
		ToolResult::Plan::Enum::Enum as Plan,
	},
	State::Struct::AphroditeState,
};

/// The narrow interpreter: one arm per plan, no policy re-tested.
///
/// Every decision was taken during selection, so this leaf only performs the
/// effects the chosen plan names. It receives no input and no policy of its own:
/// each plan carries the content it transforms and the one execution option it
/// needs, so a plan selected for one tool result cannot be executed against
/// another.
///
/// The classification hint is the path's own identity (`Hint::Tool`), which is
/// what keeps shell-shaped words in a tool result from being typed `terminal`.
pub(crate) fn Fn(State:&mut AphroditeState, Plan:Plan<'_>) -> serde_json::Value {
	match Plan {
		Plan::Passthrough(Reason) => crate::Hook::Passthrough::Fn::Fn(Reason.Text()),
		Plan::Live { Input } => {
			let Resolved = Classification(Input.Content, Input.Classify, Hint::Tool);

			Container(State, Input.Content, &Resolved.Type)
		},
		Plan::Compress { Input, AutoExpand } => Emit(State, Input.Content, Input.Classify, Hint::Tool, AutoExpand),
	}
}
