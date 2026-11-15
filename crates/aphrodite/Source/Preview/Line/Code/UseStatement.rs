//! Rust `use` statement predicate.

/// True for a Rust `use` statement ending in `;`.
///
/// A `use ` prefix, a non-empty path of alphanumerics, `_` and `::`, then `;`. It is not a Rust
/// parser: a `use` inside a `#[cfg]`-disabled block or a comment still counts, which is harmless
/// for a shape vote. The tail is taken through `str::get` because the path length is a character
/// count and the path segments may be non-ASCII.
pub(crate) fn Fn(Trimmed:&str) -> bool {
	let Rest = Trimmed.strip_prefix("use ").unwrap_or("");
	let PathLength = Rest
		.chars()
		.take_while(|Character| Character.is_alphanumeric() || *Character == '_' || *Character == ':')
		.count();

	PathLength > 0 && Rest.get(PathLength..).unwrap_or("").starts_with(';')
}
