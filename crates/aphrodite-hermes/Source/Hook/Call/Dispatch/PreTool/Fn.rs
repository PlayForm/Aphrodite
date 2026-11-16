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

use crate::Hook::Call::Input::Struct::Struct;

/// Handle `pre_tool_call`: emit the dev-only pre-tool line, then apply at most
/// one rewrite (backgrounding, then chain splitting) or pass through.
///
/// Both rewrites are values resolved BEFORE the decision to use one: each is an
/// `Option` whose `None` means "this rewrite does not apply", and `or` picks the
/// first that does. Neither can rewrite a call it does not cover, because a
/// rewrite only exists where its own guard produced one.
pub(crate) fn Fn(State:&mut AphroditeState, Input:&Struct) -> serde_json::Value {
	// DEV-ONLY debug logger (crate::Debug): pre-tool line with the tool name,
	// args byte size, effective thresholds, and the expected compression path
	// (the transform arms log the actual outcome).
	let Tool = Input.Parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
	let Bytes = Input.Parsed.get("args").map(|Args| Args.to_string().len()).unwrap_or(0);
	let Expected = crate::Configuration::Path::Fn::Fn(State, Tool, &Input.Parsed);
	crate::Debug::Log::Bridge::Fn::Fn(
		State,
		&crate::Debug::Session::Last::Fn::Fn(),
		"pre_tool",
		&[
			("tool", Tool.to_string()),
			("args_bytes", Bytes.to_string()),
			("tool_threshold", State.ToolThreshold.to_string()),
			("terminal_threshold", State.TerminalThreshold.to_string()),
			("auto_expand_limit", State.AutoExpandLimit.to_string()),
			("expected", Expected),
		],
	);

	// ── Auto-backgrounding: intercept BEFORE execution ──
	// Returns {"action":"modify","args":{"background":true,"notify_on_complete":true}}
	// so Hermes runs the tool in background instead of blocking.
	let Background = match (State.PollWorkerEnabled, Tool) {
		(true, "terminal" | "process") => {
			let Command = Input
				.Parsed
				.get("args")
				.and_then(|Args| Args.get("command"))
				.and_then(|v| v.as_str());
			// For `process(action='poll')`, don't background - it's a check call.
			let Poll = Tool == "process"
				&& Input
					.Parsed
					.get("args")
					.and_then(|Args| Args.get("action"))
					.and_then(|v| v.as_str())
					.map(|Action| Action == "poll")
					.unwrap_or(false);

			(!Poll)
				.then(|| aphrodite::PollWorker::Should::Entry::Fn::Before(Command))
				.flatten()
				.map(|(_, Summary)| {
					// We don't create a BgTask here - Hermes handles the
					// process lifecycle. We'll track completion via
					// transform_tool_result when the agent polls.
					serde_json::json!({
						"action": "modify",
						"args": {
							"background": true,
							"notify_on_complete": true,
						},
						"message": format!("aphrodite: auto-backgrounding `{}`", Summary),
					})
				})
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
	let Chain = match (State.ChainSplitEnabled, Tool) {
		(true, "terminal") => {
			Input
				.Parsed
				.get("args")
				.and_then(|Args| Args.get("command"))
				.and_then(|v| v.as_str())
				.and_then(|Command| {
					aphrodite::ChainSplit::Split::Fn::Fn(Command)
						.filter(|Segments| Segments.len() >= State.ChainSplitMinimumSegments)
						.map(|Segments| (Command, Segments))
				})
				.map(|(Command, Segments)| {
					let Rewritten = aphrodite::ChainSplit::Marked::Command::Fn::Fn(&Segments);

					(Command, Segments, Rewritten)
				})
				.filter(|(Command, _, Rewritten)| Rewritten != Command)
				.map(|(_, Segments, Rewritten)| {
					let mut Args = Input.Parsed.get("args").cloned().unwrap_or_else(|| serde_json::json!({}));
					Args["command"] = serde_json::Value::String(Rewritten);

					serde_json::json!({
						"action": "modify",
						"args": Args,
						"message": format!(
							"aphrodite: split chained command into {} segments (fine-grained CCR)",
							Segments.len()
						),
					})
				})
		},
		_ => None,
	};

	// Pass through when neither rewrite applies.
	Background.or(Chain).unwrap_or(serde_json::Value::Null)
}
