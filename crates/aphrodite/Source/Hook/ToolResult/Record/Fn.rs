//! Record a per-call telemetry event.

use super::super::Path::Fn::Fn as WrotePathFrom;
use crate::Hook::Meta::Struct::Struct as ToolCallMeta;
use crate::State::Struct::AphroditeState;
use crate::State::Event::Struct::ToolEvent;

/// Record a per-call telemetry event from the supplied metadata (P2/T6).
/// `Okay` is fail-open: a missing `Status` means success. `WrotePath` is
/// inferred from the args of `write_file`/`patch`-style tools.
///
/// The event goes into the ring for EVERY call, including ones the pipeline
/// will not compress - the phase detector and the error-loop breaker reason
/// over call shape, not over what became a marker.
pub(crate) fn Fn(State:&mut AphroditeState, ToolName:&str, Content:&str, meta:&ToolCallMeta) {
	let Okay = meta.Status.map(|s| s != "error").unwrap_or(true);
	// DEV-ONLY debug metrics (crate::debug): count tool-reported errors.
	if !Okay {
		crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::ToolErrors);
	}
	let ErrorSig = if Okay {
		None
	} else {
		Some(crate::Flow::Signature::Error::Fn::ErrorSig(meta.ErrorType, meta.ErrorMessage))
	};
	let WrotePath = WrotePathFrom(ToolName, meta.ArgsJSON, Content);
	let Sig = crate::Flow::Signature::Normalize::Fn::NormalizeArgsSig(ToolName, meta.ArgsJSON);
	State.RecordToolEvent(ToolEvent {
		Turn:State.TurnCounter,
		Tool:ToolName.to_string(),
		Sig,
		Okay,
		ErrorSig,
		bytes:Content.len(),
		WrotePath,
	});
}
