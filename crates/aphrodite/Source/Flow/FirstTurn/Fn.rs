//! The one-shot first-turn orientation.
//!
//! Rendered exactly once, on turn 0, then never again: it is the only place
//! the model is told the session runs under CCR at all. Everything it says
//! about markers is vocabulary for using them, not an explanation of how they
//! are produced.

use crate::State::Struct::AphroditeState;

/// Build the one-shot first-turn injection from the configured `SessionInject`
/// template. Returns empty string if `state.session_inject` is empty or the
/// turn has advanced past 0 (first-turn only).
///
/// Empty string = no section this turn.
pub fn Fn(State:&AphroditeState) -> String {
	// The template is the value: it is present only on turn 0 with a configured
	// injection, and the substitution runs only when it is.
	let Template = (!State.Configuration.SessionInject.is_empty() && State.Turn.Counter == 0)
		.then_some(State.Configuration.SessionInject.as_str());

	Template.map_or_else(String::new, |Template| Template.replace("{VERSION}", env!("CARGO_PKG_VERSION")))
}
