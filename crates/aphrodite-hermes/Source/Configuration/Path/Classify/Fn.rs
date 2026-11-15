//! Classify a tool call into its predicted compression path.

use aphrodite::State::Struct::AphroditeState;

use crate::Configuration::Path::{Chained, Route};

/// Mirror of `Hook::ESSENTIAL_TOOLS` (dev-only log; kept in sync deliberately -
/// drift here only mislabels a debug line).
const ESSENTIAL:&[&str] = &[
	"skill_view",
	"skills_list",
	"skill_manage",
	"memory",
	"session_search",
	"read_file",
	"read_terminal",
];

/// The predicted path for `Tool`, resolved in priority order.
///
/// The order is essential -> self-tool -> auto-expand -> chain-split ->
/// live-container -> compress-or-passthrough, and every step after the first
/// that holds is never evaluated. That laziness is the point: the chain-split
/// test walks the call's args JSON, so a call that already resolved as essential
/// or as one of our own tools must not pay for it.
pub(crate) fn Fn(State:&AphroditeState, Tool:&str, Parsed:&serde_json::Value) -> Route::Enum {
	ESSENTIAL
		.contains(&Tool)
		.then_some(Route::Enum::Essential)
		.or_else(|| (Tool.starts_with("aphrodite_") || Tool.starts_with("headroom")).then_some(Route::Enum::SelfTool))
		.or_else(|| State.AutoExpand.then_some(Route::Enum::AutoExpand))
		.or_else(|| Chained::Fn::Fn(State, Tool, Parsed).then_some(Route::Enum::ChainSplit))
		.or_else(|| State.LiveContainer.then_some(Route::Enum::LiveContainer))
		.unwrap_or(Route::Enum::CompressOrPassthrough)
}
