//! The registry state.

use crate::State::TemplatesConfiguration;

/// Process-global template registry: the parsed `[templates]` section plus
/// the `[previews] model_family` selector (default `"code_first"` when
/// unset). An unknown family degrades to `None` - the caller keeps the
/// hardcoded builders, never a panic.
///
/// Both halves live in one struct because they are read together on every
/// render: the family selects which map inside the templates is consulted, so
/// two separate globals could be observed mid-reload as a family from one
/// configuration and templates from another.
pub(crate) struct Struct {
	pub(crate) Templates:Option<TemplatesConfiguration>,
	pub(crate) Family:Option<String>,
}
