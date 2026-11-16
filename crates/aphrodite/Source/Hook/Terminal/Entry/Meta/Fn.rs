//! The terminal entry point that also carries Hermes' call metadata.
//!
//! Same pipeline as `Entry::Classify`, plus the Hermes-supplied `command` and
//! `Returncode` recorded into the `Event::Recent` telemetry ring - the signals the
//! error-loop breaker reads. `returncode == 0` (or absent) is `Okay`.

use crate::{
	Hook::{
		Outcome::Record::Fn::Fn as RecordOutcome,
		Terminal::{Entry::Span::Fn::Fn as TerminalSpan, Inner::Fn::Fn as Transform},
	},
	State::Struct::AphroditeState,
};

/// Transform terminal output with a classification and the call metadata.
///
/// Wire contract: the payload shape is `Entry::Classify`'s; `command` `None` and
/// `Returncode` `None` mean Hermes reported neither, which is `Okay`.
pub fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
	Command:Option<&str>,
	Returncode:Option<i64>,
) -> serde_json::Value {
	let Span = TerminalSpan(Content.len(), Command, Returncode);
	let _Entered = Span.enter();
	let Payload = Transform(State, Content, Classify, Command, Returncode);
	RecordOutcome(&Span, &Payload);
	Payload
}
