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
		("tool_threshold", State.Configuration.ToolThreshold.to_string()),
		("terminal_threshold", State.Configuration.TerminalThreshold.to_string()),
		("inline_byte_budget", State.Store.Budget.to_string()),
		("engine_threshold_pct", State.Configuration.EngineThresholdPct.to_string()),
		("engine_min_msgs", State.Configuration.EngineMinimumMsgs.to_string()),
		("engine_protect_first", State.Configuration.EngineProtectFirst.to_string()),
		("engine_protect_last", State.Configuration.EngineProtectLast.to_string()),
		("auto_expand", State.Configuration.AutoExpand.to_string()),
		("auto_expand_limit", State.Configuration.AutoExpandLimit.to_string()),
		("live_container", State.Configuration.LiveContainer.to_string()),
		("recursive_depth", State.Configuration.RecursiveDepth.to_string()),
		("chain_split", State.ChainSplit.Enable.to_string()),
		("flow_budget_chars", State.Configuration.FlowBudgetChars.to_string()),
		("directives", State.Directive.Active.join(",")),
		(
			"templates",
			// `Some`/`None` is the decision, not a boolean derived from it: a
			// registered template set renders its own families, and an absent one
			// means the hardcoded builders are in use.
			match State.Configuration.Templates {
				Some(_) => "registered",
				None => "builtin",
			}
			.to_string(),
		),
		("debug_level", State.Configuration.DebugLevel.to_string()),
	]
}
