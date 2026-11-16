//! `ApplyUpdate`: a parsed write to session state, or the reason there is none.

use crate::{
	ABI::Configuration::{Outcome::Enum::Enum as Outcome, Update::Enum::Enum as Update},
	State::Struct::AphroditeState,
};

/// Apply a parsed write, or pass through the reason it was not parsed.
///
/// The match is on the write's variants, so adding a settable key is a compile
/// error here rather than a silently ignored arm - which is exactly what the
/// `_ => {}` this replaces could not give. The returned outcome is the write's
/// fate, not an error: the export answers `{"status":"ok"}` in all three cases
/// because the wire envelope has always done so.
pub(crate) fn Fn(State:&mut AphroditeState, Parsed:Result<Update, Outcome>) -> Outcome {
	match Parsed {
		Ok(Update::Model(Value)) => {
			State.Model = Value;

			Outcome::Applied
		},
		Ok(Update::EngineThresholdPct(Value)) => {
			State.EngineThresholdPct = Value;

			Outcome::Applied
		},
		Ok(Update::ToolThreshold(Value)) => {
			State.ToolThreshold = Value;

			Outcome::Applied
		},
		Ok(Update::ContextEngineEnabled(Value)) => {
			State.ContextEngineEnabled = Value;

			Outcome::Applied
		},
		Err(Outcome) => Outcome,
	}
}
