//! Rust `use` statement predicate.

/// True for a Rust `use` statement ending in `;`.
///
/// A `use ` prefix, a non-empty path of alphanumerics, `_` and `::`, then `;`.
/// It is not a Rust parser: a `use` inside a `#[cfg]`-disabled block or a
/// comment still counts, which is harmless for a shape vote.
pub(crate) fn Fn(t:&str) -> bool {
	let Rest = match t.strip_prefix("use ") {
		Some(r) => r,
		None => return false,
	};
	let PathLength = Rest
		.chars()
		.take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
		.count();
	PathLength > 0 && Rest[PathLength..].starts_with(';')
}
