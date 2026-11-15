//! The pre-LLM-call hook.

use crate::State::Struct::AphroditeState;

/// Pre-LLM call hook - inject catalog + active directives into context.
///
/// Keeps its historical JSON shape (`directives`/`context`/`compressed_count`)
/// for proxy/FFI consumers, but ALSO returns the unified `Context` string built
/// by `Flow::TurnContext::Fn::BuildTurnContext` (report 05, P1) so this path and
/// the Hermes bridge path can never fork on what the model actually sees. The
/// bridge and `context_engine_pre_llm` return only `{"context": ...}`; core keeps
/// the richer shape for back-compat.
///
/// An absent `directives` or `context` is `null` rather than `""`: the bridge
/// distinguishes "nothing to inject" from "an empty injection", and a caller
/// that had to test the string would be re-deciding what this function already
/// knows.
pub fn Fn(State:&mut AphroditeState) -> serde_json::Value {
	let Span = tracing::info_span!(
		"hook.pre_llm",
		turn = State.TurnCounter,
		markers = tracing::field::Empty,
		context_bytes = tracing::field::Empty,
	);
	let _Entered = Span.enter();
	// Poll-worker checkpoint: push nudges for running/completed/failed tasks
	// (only when the master flag is enabled).
	let _ = State
		.PollWorkerEnabled
		.then(|| crate::PollWorker::Check::Fn::CheckBgTasks(State));

	let Directives = crate::Directive::Build::Fn(&State.Directives, &State.ActiveDirectives);
	// catalog_summary is called inside build_turn_context - not separately
	// (04-F3: a double call skipped delta tracking every other turn).
	let Context = crate::Flow::TurnContext::Fn::BuildTurnContext(State, None);
	let DirectivesField = (!Directives.is_empty()).then_some(Directives);
	let ContextField = (!Context.is_empty()).then_some(Context);
	let Payload = serde_json::json!({
		"status": "ok",
		"compressed_count": State.RecentMarkers.len(),
		"directives": DirectivesField,
		"context": ContextField,
	});
	// Outcome: what this turn's context actually carried. Byte counts, never the
	// context itself - it is prompt text and may quote anything the tools
	// returned.
	Span.record("markers", State.RecentMarkers.len());
	Span.record(
		"context_bytes",
		Payload
			.get("context")
			.and_then(|Value| Value.as_str())
			.map(str::len)
			.unwrap_or(0),
	);

	Payload
}
