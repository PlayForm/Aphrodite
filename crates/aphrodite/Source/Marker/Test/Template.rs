//! The TOML-driven marker block templates (pre-Rust port of
//! `_core/template.py` `_render_marker_tmpl`) and `{var}` substitution.
//!
//! Each test registers a template, renders one marker, asserts the exact
//! string, and resets the registration - the templates are process-global, so
//! a test that leaves one registered changes what every later test sees. Each
//! test is named for the one case it covers; the module path names the subject.

use crate::Marker::Format::{
	Entry::Fn as Marker,
	Registration::{Guard::Fn::Fn as Serialise, Set::Fn::Fn as Register},
	Vars::Fn as Substitute,
};

/// A configured `[templates.marker] format` wins over the fallback grammar, and
/// `{center_seg}` / `{hint}` land where the template puts them (Python parity).
#[test]
fn Configured() {
	let _Serialise = Serialise();
	let Format = "{preview}{hint}\n[{type}: {meta}{center_seg}]\n<<<CCR:{hash}|{type}|{size}>>>";
	Register(Some(Format), Some("...(use aphrodite_retrieve)"), true);
	let Rendered = Marker(
		"abc123def456abc123def456abc123def456",
		"code_rust",
		1234,
		"[code_rust:3fns 42L]",
		None,
		Some(&std::collections::BTreeMap::from([("err".to_string(), "2".to_string())])),
		Some("mid"),
	);
	assert_eq!(
		Rendered,
		"[code_rust:3fns 42L]...(use aphrodite_retrieve)\n[code_rust: \
		 err=2;center=mid]\n<<<CCR:abc123def456abc123def456abc123def456|code_rust|1234>>>",
		"configured format must win over the fallback grammar"
	);
	Register(None, None, false);
}

/// `{hint}` resolves to `""` when `[prompts] ccr_marker_hint` is off (default) -
/// the hint text is registered but must not leak into the block.
#[test]
fn Disabled() {
	let _Serialise = Serialise();
	Register(Some("{preview}{hint}"), Some("HINT-TEXT"), false);
	let Rendered = Marker("abc123def456abc123def456abc123def456", "text", 5, "hello", None, None, None);
	assert_eq!(Rendered, "hello");
	Register(None, None, false);
}

/// Unconfigured means the fixed grammar, byte-identical to the pre-port output
/// (marker line + verbatim self-bracketed preview line).
#[test]
fn Unconfigured() {
	let _Serialise = Serialise();
	Register(None, None, false);
	let Rendered = Marker(
		"abc123def456abc123def456abc123def456",
		"text",
		12,
		"[text:53L 1913B]",
		None,
		None,
		None,
	);
	assert_eq!(
		Rendered,
		"<<<CCR:abc123def456abc123def456abc123def456|text|12>>>\n[text:53L 1913B]"
	);
}

/// The opt-in knob appends the hint after the preview text in the fallback
/// grammar.
#[test]
fn Enabled() {
	let _Serialise = Serialise();
	Register(None, Some("...(use aphrodite_retrieve)"), true);
	let Rendered = Marker(
		"abc123def456abc123def456abc123def456",
		"text",
		12,
		"[text:53L 1913B]",
		None,
		None,
		None,
	);
	assert_eq!(
		Rendered, "<<<CCR:abc123def456abc123def456abc123def456|text|12>>>\n[text:53L 1913B]...(use aphrodite_retrieve)",
		"opt-in knob appends the hint after the preview text"
	);
	Register(None, None, false);
}

/// Unknown/malformed placeholders degrade to literal text (never panic, never a
/// raw-template bailout) while the known vars still substitute.
#[test]
fn Unknown() {
	let _Serialise = Serialise();
	Register(Some("{preview} {typo} {size}"), None, false);
	let Rendered = Marker("abc123def456abc123def456abc123def456", "text", 7, "p", None, None, None);
	assert_eq!(Rendered, "p {typo} 7");
	Register(None, None, false);
}

/// Known names substitute, unknown ones and malformed groups stay verbatim, and
/// `{{` / `}}` escape to literal braces.
#[test]
fn Escapes() {
	let Vars = [("hash", "abc"), ("size", "42")];
	assert_eq!(Substitute("<<<CCR:{hash}|text|{size}>>>", &Vars), "<<<CCR:abc|text|42>>>");
	assert_eq!(Substitute("{typo} {hash}", &Vars), "{typo} abc");
	assert_eq!(Substitute("a{{b}}c", &Vars), "a{b}c");
	assert_eq!(Substitute("{{{hash}}}", &Vars), "{abc}");
	assert_eq!(Substitute("no placeholders", &Vars), "no placeholders");
	assert_eq!(Substitute("{hash!r}", &Vars), "{hash!r}");
	assert_eq!(Substitute("", &Vars), "");
}
