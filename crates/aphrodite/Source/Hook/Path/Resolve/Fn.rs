//! Resolve the path a write-style tool wrote.

use crate::Hook::Path::{
	Argument::Fn::Fn as FromArgument,
	Output::Fn::Fn as FromOutput,
	Resolved::Struct::Struct as Resolved,
};

/// Infer the written path for a write-style tool (P2/T6, feeds P11).
///
/// The argument wins over the output heuristic because a `patch` reply often
/// opens with a diff header rather than the path, while the arguments always
/// name the file the agent asked to touch. It answers `None` for every other
/// tool - a guessed write path would pollute the very signal it feeds - and the
/// argument is only consulted for a tool that writes, so a `read_file` call
/// with a `path` argument is never mistaken for a write.
pub(crate) fn Fn(Content:&str, Tool:&str, Args:Option<&serde_json::Value>) -> Option<Resolved> {
	matches!(Tool, "write_file" | "patch")
		.then(|| FromArgument(Args).or_else(|| FromOutput(Content, Tool)))
		.flatten()
}
