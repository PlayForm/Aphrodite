//! `[templates.prompts]` rendering (pre-Rust port of `_core/template.py`
//! `_render_prompt_tmpl`).
//!
//! The resolution order under test: a configured template wins, a missing key
//! falls back to the compiled-in string, and an unknown placeholder stays
//! verbatim instead of being swallowed.

use super::Support::WithPrompt;
use crate::{
	Flow::Prompt::{Const::Shipped, Render::Fn::Fn as Render},
	State::Struct::AphroditeState,
};

#[test]
fn Configured() {
	let s = WithPrompt("engine_offload", "offloaded {hash} keep {tail}");
	assert_eq!(
		Render(&s, "engine_offload", &[("hash", "abc123"), ("tail", "5")]),
		"offloaded abc123 keep 5"
	);
}

#[test]
fn Override() {
	let mut s = AphroditeState::default();
	s.Configuration.Templates = Some(crate::State::Template::Struct::Struct::default());
	s.Configuration
		.Templates
		.as_mut()
		.unwrap()
		.Prompts
		.insert("search_hint".into(), "custom {hash} hint".into());
	assert_eq!(Render(&s, "search_hint", &[("hash", "xyz")]), "custom xyz hint");
}

#[test]
fn Fallback() {
	let s = AphroditeState::default(); // templates = None
	assert_eq!(
		Render(&s, "engine_offload", &[("hash", "h1"), ("tail", "3")]),
		"These messages were offloaded to reduce context. Use aphrodite_retrieve(h1) if needed. The 3 messages below \
		 are your active context."
	);
	assert_eq!(
		Render(&s, "catalog_context_warn", &[("ctx", "120")]),
		"context=120 msgs - prefer catalog over scanning history"
	);
	assert_eq!(
		Render(&s, "search_hint", &[]),
		"Use aphrodite_retrieve(hash) to expand any entry."
	);
	assert_eq!(
		Render(&s, "auto_expand_guidance", &[]),
		"Tool outputs are auto-expanded - you see full content inline. If you see a CCR marker, retrieve only if the \
		 preview hints at useful content."
	);
	// The session_inject fallback is the shipped constant; {VERSION} is
	// an unknown var here and stays verbatim, so the output is identical.
	assert_eq!(Render(&s, "session_inject", &[]), Shipped);
	// Unknown key => empty string, no panic.
	assert_eq!(Render(&s, "no_such_key", &[]), "");
}

#[test]
fn Verbatim() {
	let s = WithPrompt("catalog_context_warn", "ctx={ctx} typo={typo}");
	assert_eq!(Render(&s, "catalog_context_warn", &[("ctx", "9")]), "ctx=9 typo={typo}");
}
