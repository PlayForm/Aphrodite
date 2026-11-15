//! `RenderPreview`: resolve and fill the configured template.

use crate::Template::{State::Fn::Fn as State, Substitute::Fn::Fn as Substitute, Vars::Fn::Fn as BuildVars};

/// Render the configured `[templates.preview]` template for `TypeStr` +
/// `Content`, or `None` when nothing is configured for this type (the caller
/// then runs the hardcoded per-arm builder unchanged).
///
/// Resolution (pre-Rust `_render_template`): the family map is selected by
/// `[previews] model_family` (default `"code_first"` when unset), then
/// `family[ctype]` → `reverse[ctype]` → `family[_default]` → `None`. An
/// empty resolved template also degrades to `None` (the pre-Rust falsy
/// fallback). Substitution is defensive: unknown `{variable}`s render as
/// empty strings and a malformed template never panics.
///
/// The empty-template check is a separate step from the lookup on purpose: a
/// configured-but-blank format string is a configuration mistake that must fall
/// through to the hardcoded builder, not a template that renders "".
pub fn Fn(TypeStr:&str, Content:&str) -> Option<String> {
	// DEV-ONLY debug instrumentation (crate::Debug): template-render counter
	// + verbose line at APHRODITE_DEBUG=2 (captured only when verbose).
	let T0 = (crate::Debug::Level::Get::Fn::Fn() >= 2).then(std::time::Instant::now);
	let St = State().read().unwrap_or_else(std::sync::PoisonError::into_inner);
	let Templates = St.Templates.as_ref()?;
	let Family = St.Family.clone().unwrap_or_else(|| "code_first".to_string());
	let FamilyTemplates = Templates.Preview.get(&Family)?;
	let Tmpl = FamilyTemplates
		.get(TypeStr)
		.or_else(|| Templates.Reverse.get(TypeStr).and_then(|Mapped| FamilyTemplates.get(Mapped)))
		.or_else(|| FamilyTemplates.get("_default"))?;
	if Tmpl.trim().is_empty() {
		return None;
	}
	let Vars = BuildVars(TypeStr, Content);
	let Out = Substitute(Tmpl, &Vars);
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::TemplateRenders);
	if let Some(T0) = T0 {
		crate::Debug::Log::Verbose::Fn::Fn(
			"render_preview",
			&[
				("type", TypeStr.to_string()),
				("bytes", Content.len().to_string()),
				("elapsed_us", T0.elapsed().as_micros().to_string()),
			],
		);
	}
	Some(Out)
}
