//! Build the process-wide config-seeded state template.
//!
//! One `OnceLock`, one seed. Which seed is a compile-time decision
//! (`Template::Seeded` in a real build, `Template::Plain` in a test build), so
//! this file carries no `#[cfg]` inside a function body and no pair of
//! same-named bindings to keep in step - the two seeds cannot drift into each
//! other because only one of them exists in any given build.

use std::sync::OnceLock;

use aphrodite::State::Struct::AphroditeState;

#[cfg(not(test))]
use crate::Configuration::Template::Seeded as Seed;
#[cfg(test)]
use crate::Configuration::Template::Plain as Seed;

/// Config-seeded template state, built once per process.
///
/// Every new session state is a clone of this template, so a subagent spawn
/// inherits the same thresholds/directives/templates as the parent (the
/// single-session behaviour is unchanged: the first session IS a clone of the
/// template, so one session behaves exactly as before).
///
/// The build happens OUTSIDE the session registry's lock (see
/// `Session::Registry::Get::Fn`): it reads a file and may start a thread, and the
/// registry lock is the one lock every session in the gateway shares.
pub(crate) fn Fn() -> &'static AphroditeState {
	static Built:OnceLock<AphroditeState> = OnceLock::new();

	Built.get_or_init(Seed::Fn::Fn)
}
