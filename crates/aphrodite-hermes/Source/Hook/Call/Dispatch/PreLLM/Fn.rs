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

use aphrodite::State::Struct::AphroditeState;

use crate::{
	Debug,
	Hook::{Call::Input::Struct::Struct, Log},
};

/// Build this turn's model-visible context, or `null` when there is none.
pub(crate) fn Fn(State:&mut AphroditeState, Input:&Struct) -> serde_json::Value {
	let Parsed = &Input.Parsed;
	let ArgsLength = Input.ArgsLength;

	// 05-P1/T1: route this - the ONLY pre_llm_call arm Hermes actually calls
	// in production - through the single shared `Flow::build_turn_context`
	// assembler. This makes the G1 bug class (bridge reimplements the hook
	// body and forks on every core improvement) unrepresentable.
	// `ArgsLength` is the request-size proxy for the P9 telemetry line (Hermes
	// serializes conversation_history into the kwargs JSON).
	//
	// Per-session debug: pre_llm_call is the ONE hook Hermes threads
	// `parent_session_id` through, so record session -> parent here to build
	// the tree the debug flag resolves against.
	if let (Some(Session), Some(Parent)) = (
		Parsed.get("session_id").and_then(|v| v.as_str()),
		Parsed.get("parent_session_id").and_then(|v| v.as_str()),
	) {
		Debug::Session::Record::Fn::Fn(Session, Parent);
	}

	// DEV-ONLY debug logger (crate::Debug): pre-turn bridge line
	// (turn/session/request-size); the per-section sizes and the cumulative
	// metrics are logged inside build_turn_context.
	let Session = Parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");

	match Session {
		"" => {},
		Session => {
			crate::Debug::Log::Bridge::Fn::Fn(
				State,
				Session,
				"pre_turn",
				&[
					("turn", State.TurnCounter.to_string()),
					("session", Session.to_string()),
					("est_request_bytes", ArgsLength.to_string()),
					("pending_markers", State.RecentMarkers.len().to_string()),
				],
			);
			Log::Fn::Fn(State, Session);
		},
	}

	let Context = aphrodite::Flow::TurnContext::Fn::BuildTurnContext(State, Some(ArgsLength));

	match Context.is_empty() {
		true => serde_json::Value::Null,
		false => serde_json::json!({ "context": Context }),
	}
}
