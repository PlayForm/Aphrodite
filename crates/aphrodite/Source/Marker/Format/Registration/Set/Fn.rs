//! Publishing a registration.

use crate::Marker::Format::{
	Grammar::Enum::Enum as Grammar,
	Registration::{Const::Registration, Struct::Struct},
};

/// Register the TOML-driven marker block templates.
///
/// The three arguments are the configuration surface's own shape - `[templates
/// .marker] format`, `[templates.marker] hint`, and the `[prompts]
/// ccr_marker_hint` knob - and this is the only place they are interpreted. Set
/// by the engine binary right after its configuration load, by the proxy's
/// `/reload` handler, and by `ConfigurationLoader`'s `apply_compression` at
/// dylib init.
///
/// Two collapses happen here, both of them about states nothing downstream can
/// tell apart:
///
/// - `None` format (or an absent `[templates]` section) publishes
///   `Grammar::Canonical`, so a hot reload that removes the templates cannot
///   leave a stale format in force.
/// - The hint is kept only when the knob is enabled **and** the text is
///   non-empty, so the renderer asks one question (`Hint.is_some()`) instead of
///   two, and cannot answer them inconsistently.
///
/// The whole value is published under one write, which is what makes "a render
/// sees a format from one registration and a hint from another" impossible.
pub fn Fn(Format:Option<&str>, Hint:Option<&str>, Enabled:bool) {
	let Published = Struct {
		Grammar:match Format {
			Some(Format) => Grammar::Template(Format.to_string()),
			None => Grammar::Canonical,
		},
		Hint:Hint.filter(|Hint| Enabled && !Hint.is_empty()).map(str::to_string),
	};

	*Registration.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Published;
}
