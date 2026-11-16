//! The context-engine gate and trigger.

use crate::State::Struct::AphroditeState;

/// Gate + trigger: call [`crate::Session::Engine::Fn::EngineMaybeArchive`] only
/// while the context engine is enabled. Disabled => the engine is never
/// consulted (this is the trigger's only call site) and nothing is injected.
///
/// `None` means "nothing was archived this turn" - either the flag is off or the
/// engine itself answered `None`.
pub(crate) fn Fn(State:&mut AphroditeState) -> Option<String> {
	State
		.Configuration
		.ContextEngineEnabled
		.then(|| crate::Session::Engine::Fn::Fn(State))
		.flatten()
}
