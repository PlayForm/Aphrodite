//! `fn name(` / `def name(` / `func name(` style signature predicate.

/// True for a function-signature line for the given keyword.
///
/// The caller supplies the keyword (`fn `, `def `, `func `) so one predicate serves Rust, Python
/// and Go. After the keyword there must be a non-empty name and then an opening parenthesis. It
/// is not a signature parser: the parameter list is not read, because the only question asked is
/// whether this line declares a function.
///
/// The name length is measured in characters and then used as a byte offset, so the tail is
/// taken through `str::get`: `is_alphanumeric` admits non-ASCII identifiers, and indexing with a
/// character count used to panic on `fn ä(` inside the preview path. `get` answers `None`, which
/// is the same verdict a malformed line gets.
pub(crate) fn Fn(Trimmed:&str, Keyword:&str) -> bool {
	let Rest = Trimmed.strip_prefix(Keyword).unwrap_or("").trim_start();
	let NameLength = Rest
		.chars()
		.take_while(|Character| Character.is_alphanumeric() || *Character == '_')
		.count();

	NameLength > 0 && Rest.get(NameLength..).unwrap_or("").trim_start().starts_with('(')
}
