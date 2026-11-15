//! `RecordOutcome`: annotate the entry-point span with the decision.

use tracing::Span;

/// Record a hook's decision on `Span` from the payload it returned.
///
/// The payload is the contract the model and the bridge read, so recording from
/// it cannot drift from what actually happened: `compressed` is the decision,
/// `outcome` is the gate that produced it (`below_threshold`, `essential_tool`,
/// ... or `compressed`), and `type`/`size` describe the marker that was made.
///
/// The span must have declared these fields as `tracing::field::Empty` -
/// `record` on an undeclared field is a silent no-op, so a span that forgets one
/// loses that annotation rather than failing.
pub(crate) fn Fn(Span:&Span, Payload:&serde_json::Value) {
	Span.record(
		"compressed",
		Payload.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false),
	);
	Span.record(
		"outcome",
		Payload.get("reason").and_then(|v| v.as_str()).unwrap_or("compressed"),
	);
	Span.record("ccr_type", Payload.get("type").and_then(|v| v.as_str()).unwrap_or(""));
	Span.record("size", Payload.get("size").and_then(|v| v.as_u64()).unwrap_or(0));
}
