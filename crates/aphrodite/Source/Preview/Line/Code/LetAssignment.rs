//! `let` / `const` / `static` assignment predicate.

/// The binding keywords a `let`-style assignment may open with.
const Keywords:[&str; 3] = ["let ", "const ", "static "];

/// True for a `let x =` / `const X =` / `static X =` assignment, with an optional `mut`.
///
/// The name is required and must be followed by `=`. It is not a scope analysis: an assignment
/// inside a string or a comment still matches, because the predicate votes on lines rather than
/// parsing the language. The tail is taken through `str::get` because the name length is a
/// character count and the identifiers may be non-ASCII.
pub(crate) fn Fn(Trimmed:&str) -> bool {
	let Rest = Keywords.iter().find_map(|Keyword| Trimmed.strip_prefix(Keyword)).unwrap_or("");
	let Rest = Rest.strip_prefix("mut ").unwrap_or(Rest).trim_start();
	let NameLength = Rest
		.chars()
		.take_while(|Character| Character.is_alphanumeric() || *Character == '_' || *Character == ':')
		.count();

	NameLength > 0 && Rest.get(NameLength..).unwrap_or("").trim_start().starts_with('=')
}
