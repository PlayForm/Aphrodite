//! `config_summary`: the effective configuration as kv pairs.

use crate::State::Struct::AphroditeState;

/// DEV-ONLY: config-summary kv pairs for the session-init line.
///
/// Reports the RESOLVED values the state carries, not the files on disk: the
/// question the init line answers is "what is this session actually running
/// with", and re-reading configuration here would answer a different question
/// and could disagree with the state the turn uses.
pub fn Fn(State:&AphroditeState) -> Vec<(&'static str, String)> {
	vec![
		("tool_threshold", State.ToolThreshold.to_string()),
		("terminal_threshold", State.TerminalThreshold.to_string()),
		("inline_byte_budget", State.Budget().to_string()),
		("engine_threshold_pct", State.EngineThresholdPct.to_string()),
		("engine_min_msgs", State.EngineMinimumMsgs.to_string()),
		("engine_protect_first", State.EngineProtectFirst.to_string()),
		("engine_protect_last", State.EngineProtectLast.to_string()),
		("auto_expand", State.AutoExpand.to_string()),
		("auto_expand_limit", State.AutoExpandLimit.to_string()),
		("live_container", State.LiveContainer.to_string()),
		("recursive_depth", State.RecursiveDepth.to_string()),
		("chain_split", State.ChainSplitEnabled.to_string()),
		("flow_budget_chars", State.FlowBudgetChars.to_string()),
		("directives", State.ActiveDirectives.join(",")),
		(
			"templates",
			// `Some`/`None` is the decision, not a boolean derived from it: a
			// registered template set renders its own families, and an absent one
			// means the hardcoded builders are in use.
			match State.Templates {
				Some(_) => "registered",
				None => "builtin",
			}
			.to_string(),
		),
		("debug_level", State.DebugLevel.to_string()),
	]
}
