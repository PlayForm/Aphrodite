//! `[templates]` + `[previews] model_family` into the process-global template
//! renderer.

use super::super::super::Struct::Struct;

impl Struct {
	/// Register `[templates]` + `[previews] model_family` into the
	/// process-global template renderer (pre-Rust port of
	/// `_core/template.py::_render_template`): the consumer of
	/// `[templates.preview.<family>.<ctype>]` format strings. Called wherever
	/// `ApplyPreviews` is (dylib init + config auto-reload) so the dylib
	/// path renders TOML preview templates exactly like the engine binary and
	/// proxy `/reload` do. Absent `[templates]` = `None` = hardcoded builders
	/// remain the default; absent `ModelFamily` = `RenderPreview` defaults
	/// to `"code_first"`.
	///
	/// The family is read straight from the section, not through
	/// `GetString`: there is no env-var spelling of a preview family, and
	/// inventing one would make the family configurable in two places with
	/// different precedence.
	pub fn ApplyTemplates(&self, State:&crate::State::Struct::AphroditeState) {
		let Family = self
			.Section("previews")
			.and_then(|s| s.get("model_family"))
			.and_then(|v| v.as_str())
			.map(|s| s.to_string());
		crate::Template::Family::Set::Fn::Fn(Family);
		crate::Template::Set::Fn::Fn(State.Templates.clone());
	}
}
