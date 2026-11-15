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
pub fn BuildFirstTurnInjection(State:&AphroditeState) -> String {
	match State.SessionInject.is_empty() || State.TurnCounter > 0 {
		true => return String::new(),
		false => (),
	}

	State.SessionInject.replace("{VERSION}", env!("CARGO_PKG_VERSION"))
}
