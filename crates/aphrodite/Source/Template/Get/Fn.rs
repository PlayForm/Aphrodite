//! `GetTemplates`: the currently registered `[templates]` section.

use crate::{State::TemplatesConfiguration, Template::State::Fn::Fn as State};

/// Currently registered `[templates]` config (None = hardcoded builders).
///
/// Clones out of the lock rather than handing back a guard: the configuration
/// tables are read by callers that go on to take other locks (the proxy state,
/// the render path), and a held read guard would turn an unrelated reload into
/// a deadlock.
pub fn Fn() -> Option<TemplatesConfiguration> {
	State()
		.read()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
		.Templates
		.clone()
}
