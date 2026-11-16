//! Serializers for the process-global state the test areas share.

use crate::{ConfigurationLoader::Configuration::Struct::Struct as Configuration, State::Struct::AphroditeState};

/// Tests that mutate the process cwd / env serialize on this guard - the directive
/// applier reads cwd + env vars, so two such tests running in parallel could observe
/// each other's state.
pub(super) static CwdGuard:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

/// Serialize a state apply against the tests that read the process environment.
///
/// `ApplyCompression` writes state only - the process globals are `ApplyGlobals`'
/// business - so the guard here is about the environment and the cwd the directive
/// discovery reads, not about a global this function writes.
pub(super) fn GuardedApplyCompression(cfg:&Configuration, State:&mut AphroditeState) {
	let _G = crate::Marker::Format::Registration::Guard::Fn::Fn();

	cfg.ApplyCompression(State);
}

/// The three guards a process-global publication needs, taken in one order.
///
/// `ApplyGlobals` writes three process-wide registries and `cargo test` runs module
/// bodies concurrently: the marker registration, the template registry and the preview
/// cap each have their own guard, so one publication has to hold all three - and hold
/// them across the assertions that read the published state, which is why the guards
/// are handed back to the caller instead of being scoped to one call. Taken in this
/// order (marker, template, preview) everywhere, which is what keeps the nesting
/// deadlock-free.
pub(super) fn GlobalsGuards() -> (
	std::sync::MutexGuard<'static, ()>,
	std::sync::MutexGuard<'static, ()>,
	std::sync::MutexGuard<'static, ()>,
) {
	(
		crate::Marker::Format::Registration::Guard::Fn::Fn(),
		crate::Template::Guard::Fn::Fn(),
		crate::Preview::State::Guard::Fn(),
	)
}
