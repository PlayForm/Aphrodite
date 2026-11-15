//! Session start - full reset.

use crate::State::Struct::AphroditeState;

/// Session start hook - full reset.
///
/// Delegates to [`crate::Session::on_session_start`] rather than resetting
/// anything here: the hook is a stable entry point on the `Hook` surface (the
/// C ABI dispatch in `Library.rs` and the Hermes bridge both call it by this
/// name), and two reset implementations would drift the moment one gained a
/// field the other did not.
pub fn Fn(State:&mut AphroditeState) -> serde_json::Value {
	let Span = tracing::info_span!("hook.session_start", turn = State.TurnCounter);
	let _Entered = Span.enter();
	crate::Session::Start::Fn::OnSessionStart(State)
}
