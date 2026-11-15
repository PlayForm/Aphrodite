//! The TOML-driven marker block templates (pre-Rust port of
//! `_core/template.py` `_render_marker_tmpl`) and `{var}` substitution.
//!
//! Each test registers a template, renders one marker, asserts the exact
//! string, and resets the registration - the templates are process-global, so
//! a test that leaves one registered changes what every later test sees.

use crate::Marker::Format::{
	Const::{MarkerTemplatesTestGuard, SetMarkerTemplates},
	Entry::CCRMarker,
	Vars::SubstituteVars,
};

#[test]
fn TestMarkerFormatConfiguredRendersTomlTemplate() {
	let _G = MarkerTemplatesTestGuard();
	// Shipped-TOML-shaped template ({center_seg} for Python parity, {hint}
	// populated only when the ccr_marker_hint knob is enabled - the template
	// decides where it lands; a format without {hint} never emits it).
	let fmt = "{preview}{hint}\n[{type}: {meta}{center_seg}]\n<<<CCR:{hash}|{type}|{size}>>>";
	SetMarkerTemplates(Some(fmt), Some("...(use aphrodite_retrieve)"), true);
	let m = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"code_rust",
		1234,
		"[code_rust:3fns 42L]",
		None,
		Some(&std::collections::HashMap::from([("err".to_string(), "2".to_string())])),
		Some("mid"),
	);
	assert_eq!(
		m,
		"[code_rust:3fns 42L]...(use aphrodite_retrieve)\n[code_rust: \
		 err=2;center=mid]\n<<<CCR:abc123def456abc123def456abc123def456|code_rust|1234>>>",
		"configured format must win over the fallback grammar"
	);
	SetMarkerTemplates(None, None, false);
}

#[test]
fn TestMarkerHintVarEmptyWhenKnobDisabled() {
	let _G = MarkerTemplatesTestGuard();
	// {hint} resolves to "" when ccr_marker_hint is off (default) - the
	// hint text is registered but must not leak into the block.
	SetMarkerTemplates(Some("{preview}{hint}"), Some("HINT-TEXT"), false);
	let m = CCRMarker("abc123def456abc123def456abc123def456", "text", 5, "hello", None, None, None);
	assert_eq!(m, "hello");
	SetMarkerTemplates(None, None, false);
}

#[test]
fn TestMarkerFallbackByteIdenticalWhenUnconfigured() {
	let _G = MarkerTemplatesTestGuard();
	// Unconfigured => the fixed grammar, byte-identical to the pre-port
	// output (marker line + verbatim self-bracketed preview line).
	SetMarkerTemplates(None, None, false);
	let m = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"text",
		12,
		"[text:53L 1913B]",
		None,
		None,
		None,
	);
	assert_eq!(m, "<<<CCR:abc123def456abc123def456abc123def456|text|12>>>\n[text:53L 1913B]");
}

#[test]
fn TestMarkerFallbackAppendsHintOnlyWhenKnobEnabled() {
	let _G = MarkerTemplatesTestGuard();
	SetMarkerTemplates(None, Some("...(use aphrodite_retrieve)"), true);
	let m = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"text",
		12,
		"[text:53L 1913B]",
		None,
		None,
		None,
	);
	assert_eq!(
		m, "<<<CCR:abc123def456abc123def456abc123def456|text|12>>>\n[text:53L 1913B]...(use aphrodite_retrieve)",
		"opt-in knob appends the hint after the preview text"
	);
	SetMarkerTemplates(None, None, false);
}

#[test]
fn TestMarkerTemplateUnknownVarLeftVerbatim() {
	let _G = MarkerTemplatesTestGuard();
	// Unknown/malformed placeholders degrade to literal text (never panic,
	// never a raw-template bailout) - the known vars still substitute.
	SetMarkerTemplates(Some("{preview} {typo} {size}"), None, false);
	let m = CCRMarker("abc123def456abc123def456abc123def456", "text", 7, "p", None, None, None);
	assert_eq!(m, "p {typo} 7");
	SetMarkerTemplates(None, None, false);
}

#[test]
fn TestSubstituteVarsKnownUnknownAndEscapes() {
	let Vars = [("hash", "abc"), ("size", "42")];
	assert_eq!(SubstituteVars("<<<CCR:{hash}|text|{size}>>>", &Vars), "<<<CCR:abc|text|42>>>");
	assert_eq!(SubstituteVars("{typo} {hash}", &Vars), "{typo} abc");
	assert_eq!(SubstituteVars("a{{b}}c", &Vars), "a{b}c");
	assert_eq!(SubstituteVars("{{{hash}}}", &Vars), "{abc}");
	assert_eq!(SubstituteVars("no placeholders", &Vars), "no placeholders");
	assert_eq!(SubstituteVars("{hash!r}", &Vars), "{hash!r}");
	assert_eq!(SubstituteVars("", &Vars), "");
}
