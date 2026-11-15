//! `let` / `const` / `static` assignment predicate.

/// True for a `let x =` / `const X =` / `static X =` assignment, with an
/// optional `mut`.
///
/// The name is required and must be followed by `=`. It is not a scope
/// analysis: an assignment inside a string or a comment still matches, because
/// the predicate votes on lines rather than parsing the language.
pub(crate) fn Fn(t:&str) -> bool {
	let Rest = match ["let ", "const ", "static "].iter().find_map(|p| t.strip_prefix(p)) {
		Some(r) => r,
		None => return false,
	};
	let Rest = Rest.strip_prefix("mut ").unwrap_or(Rest).trim_start();
	let NameLength = Rest
		.chars()
		.take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
		.count();
	NameLength > 0 && Rest[NameLength..].trim_start().starts_with('=')
}
