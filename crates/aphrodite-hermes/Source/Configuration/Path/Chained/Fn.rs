//! Whether a tool call is a chained terminal command.

use aphrodite::State::Struct::AphroditeState;
use serde_json::Value;

/// True when `Tool` is a `terminal` call carrying a `&&` chain, with chain
/// splitting enabled for this session.
///
/// The tool and the flag are tested before the args are parsed, so the JSON walk
/// only happens for the one tool that can ever be chained.
pub(crate) fn Fn(State:&AphroditeState, Tool:&str, Parsed:&serde_json::Value) -> bool {
	(Tool == "terminal" && State.ChainSplitEnabled)
		&& Parsed
			.get("args")
			.and_then(|Args| Args.get("command"))
			.and_then(Value::as_str)
			.is_some_and(|Command| Command.contains("&&"))
}
