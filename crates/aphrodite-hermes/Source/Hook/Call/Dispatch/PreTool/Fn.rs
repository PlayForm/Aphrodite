//! The `pre_tool_call` arm: the one hook that can rewrite a call before it runs.
//!
//! Two rewrites live here, both returning `{"action": "modify", "args": ...}`:
//! auto-backgrounding a long command, and rewriting a chained command with
//! per-segment markers so `transform_tool_result` can split the output later.
//! Everything else returns `null` (pass through). A rewrite is the only way this
//! hook can change what Hermes executes, so each branch must be provably
//! narrower than the original call - never rewrite a call the feature does not
//! cover.
//!
//! `process(action='poll')` is deliberately excluded from backgrounding: it is
//! already a check call, and backgrounding it would replace a cheap poll with a
//! process handle.

use aphrodite::State::AphroditeState;

use crate::{Configuration::Path::Fn::expected_path, Hook::Call::Input::Struct::HookInput};

/// Handle `pre_tool_call`: emit the dev-only pre-tool line, then apply at most
/// one rewrite (backgrounding, then chain splitting) or pass through.
pub(crate) fn pre_tool_call(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	// DEV-ONLY debug logger (crate::Debug): pre-tool line with the tool name,
	// args byte size, effective thresholds, and the expected compression path
	// (the transform arms log the actual outcome).
	let call_tool = input.parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
	let args_bytes = input.parsed.get("args").map(|a| a.to_string().len()).unwrap_or(0);
	let expected = expected_path(state, call_tool, &input.parsed);
	crate::Debug::Log::Bridge::Fn(
		state,
		&crate::Debug::Session::Last::Fn(),
		"pre_tool",
		&[
			("tool", call_tool.to_string()),
			("args_bytes", args_bytes.to_string()),
			("tool_threshold", state.ToolThreshold.to_string()),
			("terminal_threshold", state.TerminalThreshold.to_string()),
			("auto_expand_limit", state.AutoExpandLimit.to_string()),
			("expected", expected),
		],
	);
	// ── Auto-backgrounding: intercept BEFORE execution ──
	// Returns {"action":"modify","args":{"background":true,"notify_on_complete":true}}
	// so Hermes runs the tool in background instead of blocking.
	if state.PollWorkerEnabled {
		let call_tool = input.parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
		if call_tool == "terminal" || call_tool == "process" {
			let command = input.parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str());
			// For `process(action='poll')`, don't background - it's a check call.
			let is_poll = call_tool == "process"
				&& input
					.parsed
					.get("args")
					.and_then(|a| a.get("action"))
					.and_then(|v| v.as_str())
					.map(|a| a == "poll")
					.unwrap_or(false);
			if !is_poll
				&& let Some((_task_id, cmd_summary)) =
					aphrodite::PollWorker::Should::Entry::Fn::ShouldBackgroundPre(command)
			{
				// We don't create a BgTask here - Hermes handles the
				// process lifecycle. We'll track completion via
				// transform_tool_result when the agent polls.
				return serde_json::json!({
					"action": "modify",
					"args": {
						"background": true,
						"notify_on_complete": true,
					},
					"message": format!("aphrodite: auto-backgrounding `{}`", cmd_summary),
				});
			}
		}
	}
	// ── Fine-grained chain splitting: rewrite chained commands ──
	// LLMs chain (`cd x && cargo build && cargo test`) into one call;
	// rewriting with segment markers lets transform_tool_result split
	// the output into per-segment CCR entries (N compact previews).
	// Tier 1 teaching loop: only chains with at least
	// `chain_split_min_segments` segments are rewritten - the
	// threshold adapts from retrieval consequences (invisible).
	if state.ChainSplitEnabled {
		let call_tool = input.parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
		if call_tool == "terminal"
			&& let Some(command) = input.parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str())
			&& let Some(segments) = aphrodite::ChainSplit::Split::Fn::Fn(command)
			&& segments.len() >= state.ChainSplitMinimumSegments
		{
			let rewritten = aphrodite::ChainSplit::Marked::Command::Fn::Fn(&segments);
			if rewritten != command {
				let mut args = input.parsed.get("args").cloned().unwrap_or_else(|| serde_json::json!({}));
				args["command"] = serde_json::Value::String(rewritten.clone());
				return serde_json::json!({
					"action": "modify",
					"args": args,
					"message": format!(
						"aphrodite: split chained command into {} segments (fine-grained CCR)",
						segments.len()
					),
				});
			}
		}
	}
	serde_json::Value::Null // pass through
}
