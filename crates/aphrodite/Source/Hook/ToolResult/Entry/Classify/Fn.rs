//! The tool-result entry point that accepts a caller-supplied classification.
//!
//! A caller that has already found the "real" payload underneath a wrapper the
//! core classifier cannot see through (an agent-specific JSON envelope, say)
//! supplies it as `(content_to_classify, type)`. Core stays agnostic to what a
//! wrapper looks like - it only ever hashes and stores the ORIGINAL `Content`, so
//! `aphrodite_retrieve` always returns exactly what was passed in; the pair
//! affects only the reported `type` and the generated preview.

use crate::{
	Hook::{
		Meta::Struct::Struct as Call,
		Outcome::Record::Fn::Fn as RecordOutcome,
		ToolResult::{Entry::Span::Fn::Fn as ToolSpan, Inner::Fn::Fn as Transform},
	},
	State::Struct::AphroditeState,
};

/// Transform tool output with a caller-supplied classification.
///
/// Wire contract: the payload shape is `Entry::Fn`'s; only the reported `type`
/// and the preview can differ. `Classify` `None` = detect.
pub fn Fn(State:&mut AphroditeState, Content:&str, ToolName:&str, Classify:Option<(&str, &str)>) -> serde_json::Value {
	let Span = ToolSpan(ToolName, Content.len());
	let _Entered = Span.enter();
	let Payload = Transform(State, Content, ToolName, Classify, &Call::default());
	RecordOutcome(&Span, &Payload);
	Payload
}
