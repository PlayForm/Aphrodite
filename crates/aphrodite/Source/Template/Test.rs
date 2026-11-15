//! The Template test battery: family/reverse/default resolution, the registry
//! round trip, and the end-to-end hand-off to `Preview::build_preview`.
//!
//! Every test mutates the process-global registry, so every test holds
//! [`TemplatesTestGuard`]. The third test also takes `Preview`'s own cap
//! guard, because it sets `PreviewMaximumChars` - the two globals are independent
//! and a test that took only one would race the other module's tests.

use std::collections::HashMap;

use crate::{
	State::TemplatesConfiguration,
	Template::{
		Family::{Get::Fn::Fn as GetPreviewFamily, Set::Fn::Fn as SetPreviewFamily},
		Get::Fn::Fn as GetTemplates,
		Guard::Fn::Fn as TemplatesTestGuard,
		Render::Fn::Fn as RenderPreview,
		Set::Fn::Fn as SetTemplates,
	},
};

fn Fam(Entries:&[(&str, &str)]) -> HashMap<String, String> {
	Entries.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

/// Family resolution + reverse map + `_default` fallback + unknown-family
/// fall-through + unknown-variable degradation, in one battery.
#[test]
fn TestRenderPreviewFamilyReverseDefaultUnknownVar() {
	let _G = TemplatesTestGuard();
	let mut Preview = HashMap::new();
	Preview.insert(
		"code_first".to_string(),
		Fam(&[
			("diff", "{type}:+{plus}/-{minus} {ln}L [{zzz}]"),
			("build", "[build {err}err {warn}warn {msg}]"),
			("grep", "[grep {items} hits {nope}]"),
			("_default", "[{type} {ln}L {size}B]"),
		]),
	);
	let mut Reverse = HashMap::new();
	Reverse.insert("build_output".to_string(), "build".to_string());
	SetTemplates(Some(TemplatesConfiguration {
		Preview,
		Marker:HashMap::new(),
		Prompts:HashMap::new(),
		Reverse,
	}));
	SetPreviewFamily(Some("code_first".to_string()));

	// family[ctype] direct hit, with an unknown {zzz} degrading to "".
	let r = RenderPreview("diff", "diff --git a/x b/x\n+a\n+b\n-c\n").expect("diff template");
	assert_eq!(r, "diff:+2/-1 4L []");

	// reverse[ctype] mapped hit: build_output -> build.
	let r = RenderPreview("build_output", "error: boom\nwarning: meh\n").expect("reverse map");
	assert_eq!(r, "[build 1err 1warn error: boom]");

	// family[_default] fallback: ls has no entry, _default renders.
	let r = RenderPreview("ls", "a.txt\nb.txt\n").expect("_default fallback");
	assert_eq!(r, "[ls 2L 12B]");

	// Unknown variables degrade to empty string.
	let r = RenderPreview("grep", "hit\n").expect("grep template");
	assert_eq!(r, "[grep ? hits ]");

	// Unknown family -> None (caller falls back to the hardcoded builder).
	SetPreviewFamily(Some("nonexistent".to_string()));
	assert!(RenderPreview("diff", "x").is_none());

	// Family unset -> default "code_first" (the registered family). No
	// +/- lines => pre-Rust defaults "?"/"?".
	SetPreviewFamily(None);
	assert_eq!(RenderPreview("diff", "a\nb\nc\n").as_deref(), Some("diff:+?/-? 3L []"));

	// No templates registered -> None.
	SetTemplates(None);
	assert!(RenderPreview("diff", "x").is_none());

	// Restore globals for later tests.
	SetPreviewFamily(None);
}

/// `SetTemplates`/`GetTemplates` round-trip + `GetPreviewFamily`.
#[test]
fn TestTemplatesRegistryRoundtrip() {
	let _G = TemplatesTestGuard();
	SetTemplates(None);
	assert!(GetTemplates().is_none());
	let mut Preview = HashMap::new();
	Preview.insert("code_first".to_string(), Fam(&[("_default", "[{type} {ln}L]")]));
	let cfg = TemplatesConfiguration { Preview, Marker:HashMap::new(), Prompts:HashMap::new(), Reverse:HashMap::new() };
	let Previous = SetTemplates(Some(cfg));
	assert!(Previous.is_none());
	assert!(GetTemplates().is_some());
	assert_eq!(SetPreviewFamily(Some("balance".to_string())), None);
	assert_eq!(GetPreviewFamily().as_deref(), Some("balance"));
	SetTemplates(None);
	SetPreviewFamily(None);
}

/// End-to-end (task deliverable 2): `BuildPreview` consults the
/// registered templates FIRST, applies the `PreviewMaximumChars` cap to the
/// rendered result, and falls back to the hardcoded per-arm dispatch when
/// nothing is registered.
#[test]
fn TestBuildPreviewConsultsRegisteredTemplates() {
	let _G = TemplatesTestGuard();
	let _Cap = crate::Preview::PreviewCapTestGuard();
	let mut FamMap = HashMap::new();
	FamMap.insert("diff".to_string(), "[tpl {type} {ln}L]".to_string());
	let mut Preview = HashMap::new();
	Preview.insert("code_first".to_string(), FamMap);
	SetTemplates(Some(TemplatesConfiguration {
		Preview,
		Marker:HashMap::new(),
		Prompts:HashMap::new(),
		Reverse:HashMap::new(),
	}));
	SetPreviewFamily(Some("code_first".to_string()));

	// Template renders first; the cap (8 chars) truncates it on a char
	// boundary, keeping room for the three-char `...` marker and the
	// closing `]`.
	crate::Preview::SetPreviewMaximumChars(Some(8));
	let Out = crate::Preview::BuildPreview("diff", "a\nb\n");
	assert_eq!(Out, "[tpl...]");

	// Unregistered -> hardcoded diff arm. Lift the cap first: at 8 chars
	// the three-char `...` leaves only `[dif`, which cannot distinguish
	// the arm that rendered from any other.
	SetTemplates(None);
	crate::Preview::SetPreviewMaximumChars(None);
	let Out = crate::Preview::BuildPreview("diff", "a\nb\n");
	assert!(Out.starts_with("[diff"), "hardcoded fallback expected, got {Out:?}");

	// Restore globals for later tests.
	crate::Preview::SetPreviewMaximumChars(None);
	SetPreviewFamily(None);
}
