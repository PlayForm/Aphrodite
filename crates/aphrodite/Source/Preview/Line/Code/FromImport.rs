//! Python `from x import y` predicate.

/// True for a Python `from <module> import ...` line.
///
/// The module name is required and must be followed by `import`. It is not a general import
/// detector - a bare `import x` is recognised by the statement predicate, which owns the whole
/// set of statement shapes. The tail is taken through `str::get` because the module length is a
/// character count and Python module names may be non-ASCII.
pub(crate) fn Fn(Trimmed:&str) -> bool {
	let Rest = Trimmed.strip_prefix("from ").unwrap_or("");
	let ModuleLength = Rest
		.chars()
		.take_while(|Character| Character.is_alphanumeric() || *Character == '_')
		.count();

	ModuleLength > 0 && Rest.get(ModuleLength..).unwrap_or("").trim_start().starts_with("import")
}
