//! Python `from x import y` predicate.

/// True for a Python `from <module> import ...` line.
///
/// The module name is required and must be followed by `import`. It is not a
/// general import detector - a bare `import x` is recognised by the statement
/// predicate, which owns the whole set of statement shapes.
pub(crate) fn Fn(t:&str) -> bool {
	let Rest = match t.strip_prefix("from ") {
		Some(r) => r,
		None => return false,
	};
	let ModLength = Rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
	ModLength > 0 && Rest[ModLength..].trim_start().starts_with("import")
}
