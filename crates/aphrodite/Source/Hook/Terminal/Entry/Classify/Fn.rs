//! The terminal entry point that accepts a caller-supplied classification.
//!
//! The same pipeline as `Entry::Fn`, with the caller's `(content_to_classify,
//! type)` pair instead of a detected type. Core still hashes and stores the
//! ORIGINAL `Content`, so `aphrodite_retrieve` returns exactly what was passed
//! in; the pair affects only the reported `type` and the generated preview.

use crate::{
	Hook::{
		Outcome::Record::Fn::Fn as RecordOutcome,
		Terminal::{Entry::Span::Fn::Fn as TerminalSpan, Inner::Fn::Fn as Transform},
	},
	State::Struct::AphroditeState,
};

/// Transform terminal output with a caller-supplied classification.
///
/// Wire contract: the payload is byte-identical in shape to `Entry::Fn`'s; only
/// the reported `type` and the preview can differ. `Classify` `None` = detect.
pub fn Fn(State:&mut AphroditeState, Content:&str, Classify:Option<(&str, &str)>) -> serde_json::Value {
	let Span = TerminalSpan(Content.len(), None, None);
	let _Entered = Span.enter();
	let Payload = Transform(State, Content, Classify, None, None);
	RecordOutcome(&Span, &Payload);
	Payload
}
