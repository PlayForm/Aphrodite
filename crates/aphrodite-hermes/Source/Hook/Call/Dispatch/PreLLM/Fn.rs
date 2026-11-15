//! The `pre_llm_call` arm: the ONE hook Hermes calls on every turn.
//!
//! It routes through the single shared `Flow::build_turn_context` assembler -
//! the same one core's `Hook::pre_llm_call` and `context_engine_pre_llm` use -
//! instead of reimplementing the hook body here. That is deliberate: a bridge
//! that composed its own context forked from core on every improvement, and
//! directives were dead on this path for two releases (01-F3) as a result. There
//! must be exactly one place that composes model-visible context.
//!
//! `pre_llm_call` is also the only hook Hermes threads `parent_session_id`
//! through (`turn_context.py:698-706`), so the session->parent edge the debug
//! flag resolves against is recorded here and nowhere else.

use aphrodite::State::AphroditeState;

use crate::{
	Debug,
	Hook::{Call::Input::Struct::HookInput, Log::Fn::log_session_init},
};

/// Build this turn's model-visible context, or `null` when there is none.
pub(crate) fn pre_llm_call(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	let parsed = &input.parsed;
	let args_len = input.args_len;

	// 05-P1/T1: route this - the ONLY pre_llm_call arm Hermes actually calls
	// in production - through the single shared `Flow::build_turn_context`
	// assembler. This makes the G1 bug class (bridge reimplements the hook
	// body and forks on every core improvement) unrepresentable.
	// `args_len` is the request-size proxy for the P9 telemetry line (Hermes
	// serializes conversation_history into the kwargs JSON).
	//
	// Per-session debug: pre_llm_call is the ONE hook Hermes threads
	// `parent_session_id` through, so record session -> parent here to build
	// the tree the debug flag resolves against.
	if let (Some(sid), Some(pid)) = (
		parsed.get("session_id").and_then(|v| v.as_str()),
		parsed.get("parent_session_id").and_then(|v| v.as_str()),
	) {
		Debug::Session::Record::Fn(sid, pid);
	}
	// DEV-ONLY debug logger (crate::Debug): pre-turn bridge line
	// (turn/session/request-size); the per-section sizes and the cumulative
	// metrics are logged inside build_turn_context.
	let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
	if !sid.is_empty() {
		crate::Debug::Log::Bridge::Fn(
			state,
			sid,
			"pre_turn",
			&[
				("turn", state.TurnCounter.to_string()),
				("session", sid.to_string()),
				("est_request_bytes", args_len.to_string()),
				("pending_markers", state.RecentMarkers.len().to_string()),
			],
		);
		log_session_init(state, sid);
	}
	let context = aphrodite::Flow::BuildTurnContext(state, Some(args_len));
	if context.is_empty() {
		serde_json::Value::Null
	} else {
		serde_json::json!({ "context": context })
	}
}
