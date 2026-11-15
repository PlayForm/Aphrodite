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

use aphrodite::State::Struct::AphroditeState;

use crate::Hook::Call::Input::Struct::HookInput;

/// Handle `pre_tool_call`: emit the dev-only pre-tool line, then apply at most
/// one rewrite (backgrounding, then chain splitting) or pass through.
pub(crate) fn pre_tool_call(state:&mut AphroditeState, input:&HookInput) -> serde_json::Value {
	// DEV-ONLY debug logger (crate::Debug): pre-tool line with the tool name,
	// args byte size, effective thresholds, and the expected compression path
	// (the transform arms log the actual outcome).
	let call_tool = input.parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
	let args_bytes = input.parsed.get("args").map(|a| a.to_string().len()).unwrap_or(0);
	let expected = crate::Configuration::Path::Fn::Fn(state, call_tool, &input.parsed);
	crate::Debug::Log::Bridge::Fn::Fn(
		state,
		&crate::Debug::Session::Last::Fn::Fn(),
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
	let background = match (state.PollWorkerEnabled, call_tool) {
		(true, "terminal" | "process") => {
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

			match is_poll {
				true => None,
				false => {
					aphrodite::PollWorker::Should::Entry::Fn::ShouldBackgroundPre(command).map(
						|(_task_id, cmd_summary)| {
							// We don't create a BgTask here - Hermes handles the
							// process lifecycle. We'll track completion via
							// transform_tool_result when the agent polls.
							serde_json::json!({
								"action": "modify",
								"args": {
									"background": true,
									"notify_on_complete": true,
								},
								"message": format!("aphrodite: auto-backgrounding `{}`", cmd_summary),
							})
						},
					)
				},
			}
		},
		_ => None,
	};

	// ── Fine-grained chain splitting: rewrite chained commands ──
	// LLMs chain (`cd x && cargo build && cargo test`) into one call;
	// rewriting with segment markers lets transform_tool_result split
	// the output into per-segment CCR entries (N compact previews).
	// Tier 1 teaching loop: only chains with at least
	// `chain_split_min_segments` segments are rewritten - the
	// threshold adapts from retrieval consequences (invisible).
	let chain = match (state.ChainSplitEnabled, call_tool) {
		(true, "terminal") => {
			input
				.parsed
				.get("args")
				.and_then(|a| a.get("command"))
				.and_then(|v| v.as_str())
				.and_then(|command| {
					aphrodite::ChainSplit::Split::Fn::Fn(command)
						.filter(|segments| segments.len() >= state.ChainSplitMinimumSegments)
						.map(|segments| (command, segments))
				})
				.map(|(command, segments)| {
					let rewritten = aphrodite::ChainSplit::Marked::Command::Fn::Fn(&segments);
					(command, segments, rewritten)
				})
				.filter(|(command, _, rewritten)| rewritten != command)
				.map(|(_, segments, rewritten)| {
					let mut args = input.parsed.get("args").cloned().unwrap_or_else(|| serde_json::json!({}));
					args["command"] = serde_json::Value::String(rewritten.clone());

					serde_json::json!({
						"action": "modify",
						"args": args,
						"message": format!(
							"aphrodite: split chained command into {} segments (fine-grained CCR)",
							segments.len()
						),
					})
				})
		},
		_ => None,
	};

	match background.or(chain) {
		Some(rewrite) => rewrite,
		None => serde_json::Value::Null, // pass through
	}
}
