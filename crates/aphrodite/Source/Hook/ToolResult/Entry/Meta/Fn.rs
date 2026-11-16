//! The tool-result entry point that also carries Hermes' call metadata.
//!
//! This is the entry point the Hermes bridge routes to, so the error/args/
//! duration signals Hermes ships on every call stop being dropped: they are
//! recorded into the `ToolEvents` telemetry ring.

use crate::{
	Hook::{
		Meta::Struct::Struct as Call,
		Outcome::Record::Fn::Fn as RecordOutcome,
		ToolResult::{Entry::Span::Fn::Fn as ToolSpan, Inner::Fn::Fn as Transform},
	},
	State::Struct::AphroditeState,
};

/// Transform tool output with a classification and the call metadata.
///
/// Wire contract: the payload shape is `Entry::Classify`'s. `meta` is Hermes'
/// report for this call; `Meta::Struct::default()` means it reported nothing.
pub fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
	meta:&Call,
) -> serde_json::Value {
	let Span = ToolSpan(ToolName, Content.len());
	let _Entered = Span.enter();
	let Payload = Transform(State, Content, ToolName, Classify, meta);
	RecordOutcome(&Span, &Payload);
	Payload
}
