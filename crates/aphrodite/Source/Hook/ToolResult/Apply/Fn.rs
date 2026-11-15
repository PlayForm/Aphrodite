//! Execute a selected tool-result plan.

use crate::{
	Hook::{
		Classification::Fn::Fn as Classification,
		Live::Container::Fn::Fn as Container,
		ToolResult::{
			Emit::Fn::Fn as Emit,
			Input::Struct::Struct as Input,
			Plan::Enum::Enum as Plan,
			Policy::Struct::Struct as Policy,
		},
	},
	State::Struct::AphroditeState,
};

/// The narrow interpreter: one arm per plan, no policy re-tested.
///
/// Every decision was taken during selection, so this leaf only performs the
/// effects the chosen plan names. The one thing it does read from the policy is
/// `AutoExpand`, because the compress path has to know whether to return the
/// stored content inline - and that is a property of the plan's execution, not a
/// second decision about which plan to run.
pub(crate) fn Fn(State:&mut AphroditeState, Input:&Input<'_>, Policy:&Policy, Plan:Plan) -> serde_json::Value {
	match Plan {
		Plan::Passthrough(Reason) => crate::Hook::Passthrough::Fn::Fn(Reason.Text()),
		Plan::Live => {
			let Resolved = Classification(Input.Content, Input.Classify, false);

			Container(State, Input.Content, &Resolved.Type)
		},
		Plan::Compress => Emit(State, Input, Policy.AutoExpand),
	}
}
