//! `struct` / `enum` / `trait` declaration predicate.

/// True for a type declaration line (`struct Name`, `enum Name`,
/// `trait Name`, optionally `pub`-prefixed).
///
/// It is not a check that the declaration is well formed - a name is required
/// and nothing after it is read. Go and Python type declarations are out of
/// scope here; they are handled by their own predicates.
pub(crate) fn Fn(t:&str) -> bool {
	let Stripped = t.strip_prefix("pub ").unwrap_or(t);

	["struct ", "enum ", "trait "].into_iter().any(|Kw| {
		Stripped.strip_prefix(Kw).is_some_and(|Rest| {
			Rest.trim_start()
				.chars()
				.take_while(|c| c.is_alphanumeric() || *c == '_')
				.count()
				> 0
		})
	})
}
