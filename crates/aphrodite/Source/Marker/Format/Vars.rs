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

	loop {
		match Chars.get(i).copied() {
			Some('{') => {
				match Chars.get(i + 1) {
					// `{{` escapes to a literal brace.
					Some('{') => {
						Out.push('{');
						i += 2;
					},
					_ => {
						// Try to parse `{name}` with a clean [A-Za-z0-9_] name.
						let mut j = i + 1;
						let mut name = String::new();
						loop {
							match Chars.get(j) {
								Some(c) if c.is_ascii_alphanumeric() || *c == '_' => {
									name.push(*c);
									j += 1;
								},
								_ => break,
							}
						}
						// A substitution needs a closed, non-empty group whose name is
						// registered; anything else leaves the brace literal.
						let Value = (!name.is_empty() && Chars.get(j) == Some(&'}'))
							.then(|| Vars.iter().find(|(k, _)| *k == name.as_str()))
							.flatten()
							.map(|(_, v)| *v);

						match Value {
							Some(v) => {
								Out.push_str(v);
								i = j + 1;
							},
							None => {
								// Unknown var or malformed group: emit the opening
								// brace verbatim and keep scanning - the rest of the
								// group text is then emitted literally.
								Out.push('{');
								i += 1;
							},
						}
					},
				}
			},
			Some('}') => {
				match Chars.get(i + 1) {
					Some('}') => {
						Out.push('}');
						i += 2;
					},
					_ => {
						Out.push('}');
						i += 1;
					},
				}
			},
			Some(c) => {
				Out.push(c);
				i += 1;
			},
			None => break,
		}
	}

	Out
}
