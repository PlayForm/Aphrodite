//! The opt-in `auto_expand_guidance` line.
//!
//! Injected only while `state.auto_expand` is set: tool outputs arrive
//! expanded inline, so a CCR marker appearing anyway is worth resolving. It
//! renders through the prompt renderer, so a configured template wins over the
//! compiled-in fallback; disabled means no bytes at all.

use crate::{Flow::Prompt::Render::Fn::Fn as Render, State::Struct::AphroditeState};

/// Auto-expand guidance section (prompt template `auto_expand_guidance`,
/// pre-Rust port): while `state.auto_expand` is enabled, inject the rendered
/// guidance so the model knows tool outputs arrive expanded inline (and that
/// a CCR marker appearing anyway is worth retrieving). Rendered through
/// [`Render`] - a configured `[templates.prompts] auto_expand_guidance`
/// wins, the compiled-in fallback otherwise. Disabled (default) = no
/// guidance, current behavior; an unknown/empty template renders nothing.
pub(crate) fn Fn(State:&AphroditeState) -> String {
	// Rendering is the work the flag buys: `then` keeps it out of the disabled
	// path, and an empty render is an absent section rather than a gate.
	State
		.AutoExpand
		.then(|| Render(State, "auto_expand_guidance", &[]))
		.filter(|Text| !Text.is_empty())
		.map_or_else(String::new, |Text| format!("[auto-expand: {}]", Text))
}
