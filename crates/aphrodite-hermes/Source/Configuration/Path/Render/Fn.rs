//! Render a resolved route as the label the pre-tool log line carries.

use crate::Configuration::Path::Route;

/// The log label for `Route`.
///
/// The strings are a wire format of sorts: they appear in the dev-only
/// `pre_tool` debug line, and an operator greps for them, so they keep the
/// spelling they have always had.
pub(crate) fn Fn(Route:Route::Enum) -> &'static str {
	match Route {
		Route::Enum::Essential => "passthrough_essential",
		Route::Enum::SelfTool => "passthrough_self",
		Route::Enum::AutoExpand => "auto_expand",
		Route::Enum::ChainSplit => "chain_split",
		Route::Enum::LiveContainer => "live_container",
		Route::Enum::Threshold => "compress_or_passthrough",
	}
}
