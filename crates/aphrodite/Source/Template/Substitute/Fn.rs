//! Fill a template's `{name}` slots.

use std::collections::HashMap;

/// `{variable}` substitution. Every `{name}` is replaced by the registered value; an
/// unknown name degrades to an empty string; a lone `{` with no closing brace is
/// passed through verbatim. Never panics on any input.
///
/// A hand-rolled scanner rather than a format-string macro: the templates come from
/// configuration, so an unknown `{name}` must render as empty instead of failing, and
/// a stray `{` must survive instead of aborting the render. `{}` is preserved as
/// literal `{}` - it is a common shell idiom in a command template and the pre-Rust
/// engine left it alone.
///
/// Both of those are `match` arms on the slot's own text: `""` is the `{}` case and
/// any other name is a lookup, so the empty-name test cannot drift from the branch it
/// guards. The unterminated case is a `let ... else` because it ENDS the scan - it
/// consumes the rest of the template and leaves nothing for the loop to continue on,
/// which is what the trailing `push_str` appends.
pub(crate) fn Fn(Tmpl:&str, Vars:&HashMap<String, String>) -> String {
	let mut Out = String::with_capacity(Tmpl.len());
	let mut Rest = Tmpl;

	while let Some(Open) = Rest.find('{') {
		Out.push_str(&Rest[..Open]);
		let After = &Rest[Open + 1..];

		let Some(Close) = After.find('}') else {
			Out.push('{');
			Out.push_str(After);
			Rest = "";

			break;
		};

		match &After[..Close] {
			"" => Out.push_str("{}"),
			name => Out.push_str(Vars.get(name).map(String::as_str).unwrap_or("")),
		}

		Rest = &After[Close + 1..];
	}

	Out.push_str(Rest);
	Out
}
