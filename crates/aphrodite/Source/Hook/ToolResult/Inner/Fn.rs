//! The tool-result pipeline: observe, select, apply.

use crate::{
	Hook::{
		Meta::Struct::Struct as ToolCallMeta,
		Observation::{Apply::Fn::Fn as ApplyObservation, Tool::Fn::Fn as Observe},
		ToolResult::{
			Apply::Fn::Fn as Apply,
			Input::Struct::Struct as Input,
			Policy::Resolve::Fn::Fn as ResolvePolicy,
			Select::Fn::Fn as Select,
		},
	},
	State::Struct::AphroditeState,
};

/// The tool-result pipeline, in three steps that cannot be reordered.
///
/// 1. OBSERVE. The call happened; that is a fact, and it is recorded before any
///    decision. This is the step that used to be two inline statements with a
///    comment telling the next author to keep them above the bypass gates - one
///    for telemetry, one for file references - and the comment existed because
///    the ordering was load-bearing and easy to break. As a value applied first,
///    a new early return cannot drop either one.
/// 2. SELECT. A pure function of the snapshotted policy and the input, producing
///    a plan. Nothing here touches the session.
/// 3. APPLY. A narrow interpreter that performs the plan's effects.
///
/// Empty content is the one case with nothing to observe: no bytes, and no args
/// signal worth a telemetry row.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
	meta:&ToolCallMeta,
) -> serde_json::Value {
	let Observation = (!Content.is_empty()).then(|| Observe(Content, ToolName, meta));
	let _ = Observation.as_ref().map(|Observation| ApplyObservation(State, Observation));

	let Input = Input { Content, Tool:ToolName, Classify, Meta:meta };
	let Policy = ResolvePolicy(State, Content);
	let Plan = Select(&Policy, Input);

	Apply(State, Plan)
}
