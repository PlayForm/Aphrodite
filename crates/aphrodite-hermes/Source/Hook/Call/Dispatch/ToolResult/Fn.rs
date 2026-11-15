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

use crate::Hook::{Call::Input::Struct::HookInput, PostTool::Fn::post_tool_log, Replacement::Fn::replacement_from};

/// Compress one tool result and return the value Hermes substitutes.
pub(crate) fn transform_tool_result(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	let tool = input.tool();
	let tool_content = input.tool_content();
	let parsed = &input.parsed;

	// Poll-result tracking: update running BgTasks from
	// process(action='poll') results. Only active when poll_worker is enabled.
	match (state.PollWorkerEnabled, tool) {
		(true, "process") => {
			aphrodite::PollWorker::Update::Fn::UpdateFromPoll(state, tool, tool_content);
		},
		_ => {},
	};

	// Hermes wraps every tool result in JSON (`{"output":...,
	// "exit_code":...}`, `{"total_count":...,"matches":[...]}`,
	// etc.) - unwrap it to classify/preview the real payload,
	// but hand core the ORIGINAL content so it's what gets
	// hashed and stored (retrieval must stay lossless).
	let classify = crate::Tool::Unwrap::Fn::Fn(tool_content);
	// 05-P2/T8: parse the error/args/timing signals Hermes ships
	// on every call (previously dropped on the floor) and route
	// them into the telemetry ring via `_with_meta`.
	let meta = aphrodite::Hook::Meta::Struct::Struct {
		ArgsJSON:parsed.get("args"),
		Status:parsed.get("status").and_then(|v| v.as_str()),
		ErrorType:parsed.get("error_type").and_then(|v| v.as_str()),
		ErrorMessage:parsed.get("error_message").and_then(|v| v.as_str()),
		DurationMs:parsed.get("duration_ms").and_then(|v| v.as_u64()),
	};
	let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
	// DEV-ONLY debug logger (crate::Debug): post-tool outcome +
	// elapsed microseconds. Timing is captured only when a debug
	// channel is on for this session (quiet path: one load).
	let t0 = crate::Debug::Log::Any::Fn::Fn(state, sid).then(std::time::Instant::now);
	let r = aphrodite::Hook::ToolResult::Entry::Fn::TransformToolResultWithMeta(
		state,
		tool_content,
		tool,
		classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
		&meta,
	);
	match t0 {
		Some(t0) => {
			let us = t0.elapsed().as_micros() as u64;
			aphrodite::Debug::Timing::Add::Fn::Fn(aphrodite::Debug::Timing::Enum::Enum::ToolTransform, us);
			post_tool_log(state, sid, &r, tool_content.len(), us);
		},
		None => {},
	}

	// Per-session debug: the caller's session_id rides in the
	// hook kwargs - thread it so the flag resolves against the
	// session tree (root scoped flag wins, global fallback).
	replacement_from(&r, sid)
}
