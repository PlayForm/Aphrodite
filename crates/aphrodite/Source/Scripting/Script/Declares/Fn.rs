//! Does a compiled script declare a hook?

/// The hook name, when the compiled script declares `Hook` as a zero-argument function.
///
/// Returning the name rather than a boolean is what lets the caller treat "this script
/// opts out" as absence: a script written for a different surface is not an error, it
/// is simply not this hook's, and the caller's `let ... else` says so without a test
/// beside it.
///
/// The zero-argument check is the compatibility contract: a hook reads the seeded
/// scope, so a function taking parameters is a different function that happens to share
/// the name.
pub(crate) fn Fn<'a>(AST:&rhai::AST, Hook:&'a str) -> Option<&'a str> {
	AST.iter_functions()
		.any(|Function| Function.name == Hook && Function.params.is_empty())
		.then_some(Hook)
}
