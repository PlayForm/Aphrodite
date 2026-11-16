//! The precedence chain over the replacement outcomes.

use serde_json::Value;

use crate::Hook::Replacement::Choice::{Chain, Enum::Enum, Expanded, Marker};

/// Resolve the replacement a payload earns, first match wins.
///
/// The chain is lazy, and that is the whole shape: a payload that IS a chain
/// split never evaluates the auto-expand and marker alternatives, where the
/// tuple this replaced read all three fields of every payload and then chose by
/// tuple position with `..` wildcards. Position is now the only priority that
/// exists, and a new outcome is one `or_else` line plus one arm at the renderer.
///
/// It always answers: `Passthrough` is the absence of a replacement, so a caller
/// never has to distinguish "nothing matched" from "replace with nothing".
pub(crate) fn Fn(Result:&Value) -> Enum<'_> {
	Chain::Fn::Fn(Result)
		.map(Enum::Chain)
		.or_else(|| Expanded::Fn::Fn(Result).map(Enum::Expanded))
		.or_else(|| Marker::Fn::Fn(Result).map(Enum::Marker))
		.unwrap_or(Enum::Passthrough)
}
