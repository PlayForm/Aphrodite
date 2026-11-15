//! Arrow-function bindings.

use crate::StructExtract::Declaration::Enum::Enum as Declaration;

/// The binding keywords an arrow function is assigned through.
const Keywords:[&str; 3] = ["const ", "let ", "var "];

/// Recognise `const name = (...) => ...`.
///
/// A trailing `//` line comment is stripped BEFORE the binding is split, and the `=>` has to appear on
/// the right-hand side of the first `=` - so `const x = 5; // map => y` is a constant, not a function. A
/// naive fix that only special-cased a `//`-prefixed line would still miss that case, because the line
/// as a whole does not start with `//`. A `=>` that only appears in a trailing comment must never turn
/// a plain constant into a function, or the model is told the file exports something it does not.
///
/// It is not a JS parser: the value is never tracked across lines, so a function expression assigned on
/// one line and arrowed on the next is missed.
pub(crate) fn Fn(Trimmed:&str) -> Declaration {
	let Code = Trimmed.split("//").next().unwrap_or(Trimmed);

	match Code.split_once('=') {
		Some((Binding, Value)) => {
			let Binding = Binding.trim();
			let Named = Keywords.iter().any(|Keyword| Binding.starts_with(Keyword)) && Value.contains("=>");
			let Name = Keywords
				.iter()
				.find_map(|Keyword| Binding.strip_prefix(Keyword))
				.unwrap_or("")
				.trim();

			match Named.then(|| Name.to_string()).filter(|Name| {
				!Name.is_empty() && Name.chars().all(|Character| Character.is_alphanumeric() || Character == '_')
			}) {
				Some(Name) => Declaration::Declaration { Kind:"=>", Name, Parameters:None, Return:None },
				None => Declaration::NotDeclaration,
			}
		},
		None => Declaration::NotDeclaration,
	}
}
