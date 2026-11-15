//! `{name}` substitution for TOML templates.
//!
//! The Rust port of the pre-Rust Python `str.format(**safe)` semantics used by
//! `_core/template.py` (`_render_marker_tmpl`, `_render_prompt_tmpl`). The one
//! behavioural difference from Python is deliberate: Python raised on an
//! unknown placeholder (a crash in `_render_marker_tmpl`, a raw-template
//! fallback in `_render_prompt_tmpl`); this degrades to emitting the
//! placeholder verbatim, so a user's typo in `aphrodite.toml` leaves a visibly
//! wrong marker instead of taking the proxy down.

/// Substitute `{name}` placeholders in a template string.
///
/// - `{{` / `}}` escape to literal braces (Python str.format).
/// - Known variable names are replaced by their string values.
/// - Unknown placeholders, malformed groups (e.g. `{hash!r}`), and stray
///   braces are emitted verbatim: Python raised KeyError/ValueError (a
///   crash in `_render_marker_tmpl`, a raw-template fallback in
///   `_render_prompt_tmpl`); this crate degrades without panicking and
///   keeps the template readable.
pub(crate) fn SubstituteVars(Tmpl:&str, Vars:&[(&str, &str)]) -> String {
	let Chars:Vec<char> = Tmpl.chars().collect();
	let mut Out = String::with_capacity(Tmpl.len() + 16);
	let mut i = 0;
	while i < Chars.len() {
		let c = Chars[i];
		if c == '{' {
			if i + 1 < Chars.len() && Chars[i + 1] == '{' {
				Out.push('{');
				i += 2;
				continue;
			}
			// Try to parse `{name}` with a clean [A-Za-z0-9_] name.
			let mut j = i + 1;
			let mut name = String::new();
			while j < Chars.len() && (Chars[j].is_ascii_alphanumeric() || Chars[j] == '_') {
				name.push(Chars[j]);
				j += 1;
			}
			if !name.is_empty() && j < Chars.len() && Chars[j] == '}' {
				let Found = Vars.iter().find(|(k, _)| *k == name.as_str());
				if let Some((_, v)) = Found {
					Out.push_str(v);
					i = j + 1;
					continue;
				}
			}
			// Unknown var or malformed group: emit the opening brace
			// verbatim and keep scanning - the rest of the group text is
			// then emitted literally.
			Out.push('{');
			i += 1;
			continue;
		}
		if c == '}' {
			if i + 1 < Chars.len() && Chars[i + 1] == '}' {
				Out.push('}');
				i += 2;
				continue;
			}
			Out.push('}');
			i += 1;
			continue;
		}
		Out.push(c);
		i += 1;
	}
	Out
}
