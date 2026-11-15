//! The pre-LLM-call hook.

use crate::State::Struct::AphroditeState;

/// Pre-LLM call hook - inject catalog + active directives into context.
///
/// Keeps its historical JSON shape (`catalog`/`Directives`/`compressed_count`)
/// for proxy/FFI consumers, but ALSO returns the unified `Context` string built
/// by `Flow::build_turn_context` (report 05, P1) so this path and the Hermes
/// bridge path can never fork on what the model actually sees. The bridge and
/// `context_engine_pre_llm` return only `{"context": ...}`; core keeps the
/// richer shape for back-compat.
pub fn Fn(State:&mut AphroditeState) -> serde_json::Value {
	// Poll-worker checkpoint: push nudges for running/completed/failed tasks
	// (only when the master flag is enabled).
	match State.PollWorkerEnabled {
		true => {
			crate::PollWorker::Check::Fn::CheckBgTasks(State);
		},
		false => {},
	}
	let Directives = crate::Directive::Build::Fn(&State.Directives, &State.ActiveDirectives);
	// catalog_summary is now called inside build_turn_context - don't call
	// it separately (04-F3: double call skipped delta tracking every other turn).
	let Context = crate::Flow::TurnContext::Fn::BuildTurnContext(State, None);
	let DirectivesField = match Directives.is_empty() {
		true => None,
		false => Some(Directives),
	};
	let ContextField = match Context.is_empty() {
		true => None,
		false => Some(Context),
	};
	serde_json::json!({
		"status": "ok",
		"compressed_count": State.RecentMarkers.len(),
		"directives": DirectivesField,
		"context": ContextField,
	})
}
