//! The kinds a non-empty terminal output can be.

/// What a non-empty terminal output is.
///
/// `Build` carries the compiler-error bit rather than being split into two
/// variants, because the second test only ever runs once the first has held: an
/// output with `error: could not ...` but none of the build markers is NOT a
/// build at all, so the two decisions cannot be collapsed into one ordered list
/// without changing which outputs reach the content classifier.
pub(crate) enum Enum {
	/// A shell failure trace (`exit code:` / `Error:`).
	Failure,
	/// Cargo/test/install output, and whether the compiler actually failed.
	Build { Error:bool },
	/// No build or failure marker: the shared classifier decides.
	Detected,
}
