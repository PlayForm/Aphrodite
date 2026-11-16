//! The TOC-only fields.

use crate::{
	Catalog::{Build::Recommendation, Mode},
	State::Struct::AphroditeState,
};

/// Add the fields only a TOC catalog carries.
///
/// One decision: a `Full` catalog gets nothing, a `TOC` catalog gets its
/// recommendations and the configured `search_hint`. `for_each` over the `Option` is the
/// shape because the enrichment is a side effect on a value that may not exist - the list
/// answers `None` when there is nothing to suggest, and a `None` must leave the JSON
/// untouched rather than add an empty field.
///
/// The hint comes from `[templates.prompts] search_hint` (pre-Rust port) and falls back
/// to the compiled-in string when the key is absent, which `RenderPrompt` already
/// decides.
pub(crate) fn Fn(State:&AphroditeState, Mode:&Mode::Enum::Enum, Result:&mut serde_json::Value) {
	let Recommendations = match Mode {
		Mode::Enum::Enum::Full => None,
		Mode::Enum::Enum::TOC => Recommendation::Fn::Fn(State),
	};

	Recommendations.into_iter().for_each(|Recommendations| {
		Result["recommendations"] = serde_json::json!(Recommendations);
		Result["hint"] = serde_json::json!(crate::Flow::Prompt::Render::Fn::RenderPrompt(State, "search_hint", &[]));
	});
}
