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
/// External contract: `Template` is the `[templates.marker] format` (or
/// `[templates.prompt]`) string exactly as it appears in `aphrodite.toml`, and
/// `Vars` is the closed set of names a template may use. The result is what the
/// consumer reads, byte for byte.
///
/// - `{{` / `}}` escape to literal braces (Python str.format).
/// - Known variable names are replaced by their string values.
/// - Unknown placeholders, malformed groups (e.g. `{hash!r}`), and stray
///   braces are emitted verbatim: Python raised KeyError/ValueError (a
///   crash in `_render_marker_tmpl`, a raw-template fallback in
///   `_render_prompt_tmpl`); this crate degrades without panicking and
///   keeps the template readable.
pub(crate) fn Fn(Template:&str, Vars:&[(&str, &str)]) -> String {
	let Characters:Vec<char> = Template.chars().collect();
	let mut Rendered = String::with_capacity(Template.len() + 16);
	let mut Index = 0;

	loop {
		match Characters.get(Index).copied() {
			Some('{') => {
				match Characters.get(Index + 1) {
					// `{{` escapes to a literal brace.
					Some('{') => {
						Rendered.push('{');
						Index += 2;
					},
					_ => {
						// Try to parse `{name}` with a clean [A-Za-z0-9_] name.
						let mut End = Index + 1;
						let mut Name = String::new();
						loop {
							match Characters.get(End) {
								Some(Character) if Character.is_ascii_alphanumeric() || *Character == '_' => {
									Name.push(*Character);
									End += 1;
								},
								_ => break,
							}
						}
						// A substitution needs a closed, non-empty group whose name is
						// registered; anything else leaves the brace literal.
						let Value = (!Name.is_empty() && Characters.get(End) == Some(&'}'))
							.then(|| Vars.iter().find(|(Key, _)| *Key == Name.as_str()))
							.flatten()
							.map(|(_, Text)| *Text);

						match Value {
							Some(Text) => {
								Rendered.push_str(Text);
								Index = End + 1;
							},
							None => {
								// Unknown var or malformed group: emit the opening
								// brace verbatim and keep scanning - the rest of the
								// group text is then emitted literally.
								Rendered.push('{');
								Index += 1;
							},
						}
					},
				}
			},
			Some('}') => {
				match Characters.get(Index + 1) {
					Some('}') => {
						Rendered.push('}');
						Index += 2;
					},
					_ => {
						Rendered.push('}');
						Index += 1;
					},
				}
			},
			Some(Character) => {
				Rendered.push(Character);
				Index += 1;
			},
			None => break,
		}
	}

	Rendered
}
