//! `AphroditeTerminal` - run the terminal-output hook for one payload.

use std::os::raw::c_char;

use super::Guard::MaximumContent;

stateful!(
	/// Transform captured terminal output through the terminal hook and answer
	/// with the hook's JSON result.
	///
	/// Separate from `AphroditeTransform` because the terminal hook classifies
	/// and previews differently (exit codes and error lines are the signal, not
	/// file structure), and because `TransformTerminalOutput` has no tool name
	/// to key on.
	AphroditeTerminal,
	|s, Content:*const c_char| {
		if Content.len() > MaximumContent {
			return serde_json::json!({"error":"content exceeds 16MB limit"});
		}
		crate::Hook::TransformTerminalOutput(s, &Content)
	}
);
