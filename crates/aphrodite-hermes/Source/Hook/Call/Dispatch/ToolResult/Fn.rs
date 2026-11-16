//! The `transform_tool_result` arm: the hook Hermes fires on EVERY tool result.
//!
//! Two contracts meet here. The CLASSIFIER sees the unwrapped payload (Hermes
//! wraps every result in JSON - `{"output":..., "exit_code":...}`,
//! `{"total_count":..., "matches":[...]}` - and classifying the wrapper yields a
//! useless `[json:1items 1L]` preview), while the STORE must receive the
//! ORIGINAL content so retrieval stays lossless. Passing the unwrapped payload
//! to the store would silently make `aphrodite_retrieve` return something the
//! model never saw.

use aphrodite::State::Struct::AphroditeState;

use crate::Hook::{Call::Input::Struct::Struct, PostTool, Replacement};

/// Compress one tool result and return the value Hermes substitutes.
///
/// Two payloads leave this leaf: the CLASSIFIER sees the unwrapped fragment
/// (Hermes wraps every result in JSON, and classifying the wrapper yields a
/// useless `[json:1items 1L]` preview), while the STORE receives the ORIGINAL
/// content, so `aphrodite_retrieve` returns exactly what the model was shown.
pub(crate) fn Fn(State:&mut AphroditeState, Input:&Struct) -> serde_json::Value {
	let Tool = Input.Tool();
	let Content = Input.Content();
	let Parsed = &Input.Parsed;

	// Poll-result tracking: update running BgTasks from
	// process(action='poll') results. Only active when poll_worker is enabled.
	(State.PollWorkerEnabled && Tool == "process")
		.then_some(())
		.into_iter()
		.for_each(|()| {
			aphrodite::PollWorker::Update::Fn::Fn(State, Tool, Content);
		});

	let Classify = crate::Tool::Unwrap::Fn::Fn(Content);
	// 05-P2/T8: parse the error/args/timing signals Hermes ships
	// on every call (previously dropped on the floor) and route
	// them into the telemetry ring via `_with_meta`.
	let Meta = aphrodite::Hook::Meta::Struct::Struct {
		ArgsJSON:Parsed.get("args"),
		Status:Parsed.get("status").and_then(|v| v.as_str()),
		ErrorType:Parsed.get("error_type").and_then(|v| v.as_str()),
		ErrorMessage:Parsed.get("error_message").and_then(|v| v.as_str()),
		DurationMs:Parsed.get("duration_ms").and_then(|v| v.as_u64()),
	};
	let Session = Parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
	// DEV-ONLY debug logger (crate::Debug): post-tool outcome +
	// elapsed microseconds. Timing is captured only when a debug
	// channel is on for this session (quiet path: one load).
	let Started = crate::Debug::Log::Any::Fn::Fn(State, Session).then(std::time::Instant::now);
	let Result = aphrodite::Hook::ToolResult::Entry::Meta::Fn::Fn(
		State,
		Content,
		Tool,
		Classify.as_ref().map(|(Body, Kind)| (Body.as_str(), Kind.as_str())),
		&Meta,
	);
	Started.into_iter().for_each(|Started| {
		let Micros = Started.elapsed().as_micros() as u64;
		aphrodite::Debug::Timing::Add::Fn::Fn(aphrodite::Debug::Timing::Enum::Enum::ToolTransform, Micros);
		PostTool::Fn::Fn(State, Session, &Result, Content.len(), Micros);
	});

	// Per-session debug: the caller's session_id rides in the
	// hook kwargs - thread it so the flag resolves against the
	// session tree (root scoped flag wins, global fallback).
	Replacement::Fn::Fn(&Result, Session)
}
