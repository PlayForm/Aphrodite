//! Predict which compression path a tool call will take.
//!
//! "Path" here is the configured DECISION path, not a filesystem path: the
//! label is derived from the session's config fields (`auto_expand`,
//! `chain_split_enabled`, `live_container`) plus the tool name and its args.
//! It exists only to label the dev-only pre-tool debug line - the transform
//! arms log the ACTUAL outcome - so a wrong prediction mislabels a log line
//! and nothing else.

use aphrodite::State::AphroditeState;

/// Predict the compression path for a tool call (best-effort label for the
/// pre_tool debug line; the transform arms log the actual outcome).
pub(crate) fn expected_path(state:&AphroditeState, tool:&str, parsed:&serde_json::Value) -> String {
	// Mirror of `Hook::ESSENTIAL_TOOLS` (dev-only log; kept in sync
	// deliberately - drift here only mislabels a debug line).
	const ESSENTIAL:&[&str] = &[
		"skill_view",
		"skills_list",
		"skill_manage",
		"memory",
		"session_search",
		"read_file",
		"read_terminal",
	];
	if ESSENTIAL.contains(&tool) {
		return "passthrough_essential".into();
	}
	if tool.starts_with("aphrodite_") || tool.starts_with("headroom") {
		return "passthrough_self".into();
	}
	if state.AutoExpand {
		return "auto_expand".into();
	}
	if tool == "terminal" && state.ChainSplitEnabled {
		let command = parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str());
		if command.is_some_and(|c| c.contains("&&")) {
			return "chain_split".into();
		}
	}
	if state.LiveContainer {
		return "live_container".into();
	}
	"compress_or_passthrough".into()
}
