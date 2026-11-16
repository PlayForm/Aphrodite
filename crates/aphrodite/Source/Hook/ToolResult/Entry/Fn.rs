//! The tool-result entry point.
//!
//! One pipeline, three signatures: this leaf is the plain form, `Classify/`
//! accepts a caller-supplied classification and `Meta/` also carries Hermes' call
//! metadata. They exist as separate functions because the FFI, the proxy and the
//! bridge each know a different amount about the call; they must never fork on
//! what the pipeline does with it.
//!
//! Every form opens the same `hook.tool_result` span (`Span/`) and records the
//! pipeline's decision on it (`Hook::Outcome::Record`), so one trace line says
//! which gate answered - `below_threshold`, `essential_tool` or `compressed`.

use crate::{
	Hook::{
		Meta::Struct::Struct as Call,
		Outcome::Record::Fn::Fn as RecordOutcome,
		ToolResult::{Entry::Span::Fn::Fn as ToolSpan, Inner::Fn::Fn as Transform},
	},
	State::Struct::AphroditeState,
};

/// Transform tool output - full compression pipeline.
///
/// Wire contract: returns the payload Hermes receives (a marker object with
/// `compressed`, `hash`, `type`, `size`, `preview`, or a passthrough carrying
/// `reason`), records the decision on the `hook.tool_result` span, and mutates
/// session state (store, telemetry ring).
pub fn Fn(State:&mut AphroditeState, Content:&str, ToolName:&str) -> serde_json::Value {
	let Span = ToolSpan(ToolName, Content.len());
	let _Entered = Span.enter();
	let Payload = Transform(State, Content, ToolName, None, &Call::default());
	RecordOutcome(&Span, &Payload);
	Payload
}
