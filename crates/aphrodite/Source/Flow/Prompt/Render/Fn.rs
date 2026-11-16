//! Resolve one `[templates.prompts]` entry to its final text.
//!
//! Resolution order is configured template, then compiled-in fallback, then
//! the empty string. That last arm is deliberate: an unknown prompt name
//! renders as "" instead of panicking, because a typo in a prompt knob must
//! degrade the injected context, never take the turn down.

use crate::{Flow::Prompt::Const::Fallback, Marker::Format::Vars::Fn as Substitute, State::Struct::AphroditeState};

/// Render a `[templates.prompts]` template (pre-Rust port of
/// `_core/template.py` `_render_prompt_tmpl`).
///
/// Resolution: `templates.prompts[name]` → compiled-in fallback (see
/// [`Fallback`]). `{var}` substitution applies to whichever string
/// won, with the same degrade-don't-panic semantics as the marker templates
/// (unknown placeholders stay verbatim). Consumers: `search_hint` feeds the
/// catalog hint; `SessionInject` is resolved at config load;
/// `EngineOffload` / `auto_expand_guidance` / `catalog_context_warn` are
/// available for the engine-offload / auto-expand / context-count sites.
pub fn Fn(State:&AphroditeState, name:&str, Vars:&[(&str, &str)]) -> String {
	let Tmpl = State
		.Templates
		.as_ref()
		.and_then(|t| t.Prompts.get(name))
		.map(String::as_str)
		.unwrap_or_else(|| Compiled(name));
	Substitute(Tmpl, Vars)
}

fn Compiled(name:&str) -> &'static str { Fallback.iter().find(|(k, _)| *k == name).map(|(_, v)| *v).unwrap_or("") }
