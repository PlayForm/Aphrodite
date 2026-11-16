//! `Template::Set::Fn`: register the parsed `[templates]` section.

use crate::{State::Template::Struct::Struct as TemplatesConfiguration, Template::State::Fn::Fn as State};

/// Register the `[templates]` config (None = hardcoded builders remain the
/// default). Returns the previous value so callers (tests, hot-reload) can
/// restore it - same pattern as `Preview::State::Set`.
///
/// Returning the previous value is the whole reason this is a `set` and not an
/// `Apply`: a `/reload` that fails later can put back exactly what it replaced,
/// and a test can restore the registry without knowing what was there.
pub fn Fn(Templates:Option<TemplatesConfiguration>) -> Option<TemplatesConfiguration> {
	let mut St = State().write().unwrap_or_else(std::sync::PoisonError::into_inner);
	std::mem::replace(&mut St.Templates, Templates)
}
