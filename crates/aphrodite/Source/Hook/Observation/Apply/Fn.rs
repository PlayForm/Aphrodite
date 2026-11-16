//! Apply one observation to the session.

use crate::{
	Hook::Observation::{Outcome::Enum::Enum as Outcome, Struct::Struct},
	State::{Event::Struct::ToolEvent, Struct::AphroditeState},
};

/// Record an observation: the telemetry event and the file reference.
///
/// Two effects that are independent decisions, applied together because the
/// observation is one fact about the world. The tool event goes into the ring
/// for EVERY call the pipeline observed, including the ones it will not
/// compress - the phase detector and the error-loop breaker reason over call
/// shape, not over what became a marker.
///
/// The file reference is gated on `FileTools`, not on the essential-tool list:
/// `read_file` is in both on purpose (it is never compressed, and it is the
/// single most common file-reference source), so skipping compression and
/// recording a reference have to be decided separately. That is why this leaf
/// knows about `EssentialTools` only for the doc note above and reads
/// `State.FileTools` for the decision.
pub(crate) fn Fn(State:&mut AphroditeState, Observation:&Struct) {
	let Failed = matches!(Observation.Outcome, Outcome::Failed);

	// DEV-ONLY debug metrics (crate::debug): count tool-reported errors.
	Failed
		.then_some(crate::Debug::Count::Enum::Enum::ToolErrors)
		.into_iter()
		.for_each(crate::Debug::Count::Bump::Fn::Fn);

	State.RecordToolEvent(ToolEvent {
		Turn:State.TurnCounter,
		Tool:Observation.Tool.clone(),
		Sig:Observation.ArgsSig,
		Okay:!Failed,
		ErrorSig:Observation.ErrorSig,
		bytes:Observation.Bytes,
		WrotePath:Observation.WrotePath.as_ref().map(|Resolved| Resolved.Path.clone()),
	});

	let Tracked = State.FileTools.iter().any(|Tool| Tool == &Observation.Tool);
	let Referenced = Tracked.then_some(Observation.ReferencedPath.as_ref()).flatten();

	Referenced
		.into_iter()
		.for_each(|Resolved| State.RecordFile(Resolved.Path.clone(), Observation.Tool.clone()));
}
