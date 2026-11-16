//! The terminal-output pipeline: observe, select, apply.

use crate::{
	Hook::{
		Observation::{Apply::Fn::Fn as ApplyObservation, Terminal::Fn::Fn as Observe},
		Terminal::{
			Apply::Fn::Fn as Apply,
			Input::Struct::Struct as Input,
			Policy::Resolve::Fn::Fn as ResolvePolicy,
			Select::Fn::Fn as Select,
		},
	},
	State::Struct::AphroditeState,
};

/// The terminal-output pipeline, in three steps that cannot be reordered.
///
/// 1. OBSERVE. The call happened; that is a fact, and it is recorded before any
///    decision. Telemetry used to be a statement at the top of this function with
///    a comment telling the next author not to move it below a gate; now the
///    observation is a value applied first, so a new early return cannot drop it.
/// 2. SELECT. A pure function of the snapshotted policy and the input, producing
///    a plan. Nothing here touches the session.
/// 3. APPLY. A narrow interpreter that performs the plan's effects.
///
/// Empty content is the one case with nothing to observe: no bytes, and a
/// command whose output is empty is not a signal the error-loop breaker can use.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
	command:Option<&str>,
	Returncode:Option<i64>,
) -> serde_json::Value {
	let Observation = (!Content.is_empty()).then(|| Observe(Content, command, Returncode));
	let _ = Observation.as_ref().map(|Observation| ApplyObservation(State, Observation));

	let Input = Input { Content, Classify };
	let Policy = ResolvePolicy(State, Content);
	let Plan = Select(&Policy, Input);

	Apply(State, Plan)
}
