//! The terminal-output entry point.
//!
//! One pipeline, three signatures: this leaf is the plain form, `Classify/`
//! accepts a caller-supplied classification and `Meta/` also carries the
//! `command`/`Returncode` pair Hermes reports for a terminal call. `Returncode`
//! is what makes the error-loop breaker work: a failing command is a signal even
//! when the output is too small to compress.

use crate::{
	Hook::{
		Outcome::Record::Fn::Fn as RecordOutcome,
		Terminal::{Entry::Span::Fn::Fn as TerminalSpan, Inner::Fn::Fn as Transform},
	},
	State::Struct::AphroditeState,
};

/// Transform terminal output - exit code aware.
///
/// Wire contract: returns the payload Hermes receives (a marker object with
/// `compressed`, `hash`, `type`, `size`, `preview`, or a passthrough carrying
/// `reason`), records the decision on the `hook.terminal_output` span, and
/// mutates session state (store, telemetry ring).
pub fn Fn(State:&mut AphroditeState, Content:&str) -> serde_json::Value {
	let Span = TerminalSpan(Content.len(), None, None);
	let _Entered = Span.enter();
	let Payload = Transform(State, Content, None, None, None);
	RecordOutcome(&Span, &Payload);
	Payload
}
