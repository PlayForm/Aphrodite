//! `fn name(` / `def name(` / `func name(` style signature predicate.

/// True for a function-signature line for the given keyword.
///
/// The caller supplies the keyword (`fn `, `def `, `func `) so one predicate
/// serves Rust, Python and Go. After the keyword there must be a non-empty
/// name and then an opening parenthesis. It is not a signature parser: the
/// parameter list is not read, because the only question asked is whether this
/// line declares a function.
pub(crate) fn Fn(t:&str, Kw:&str) -> bool {
	let Rest = match t.strip_prefix(Kw) {
		Some(r) => r,
		None => return false,
	};
	let Rest = Rest.trim_start();
	let NameLength = Rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
	NameLength > 0 && Rest[NameLength..].trim_start().starts_with('(')
}
