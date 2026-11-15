//! The `+`/`-` line tallies of a diff.

/// How many lines a diff added and removed.
///
/// Two counts rather than two formatted strings: the pair is a measurement, and the
/// `"?"` the pre-Rust engine rendered for "not a diff" belongs to the renderer. A
/// tuple of `String` made the counts indistinguishable from the placeholder.
pub(crate) struct Struct {
	/// Lines starting with `+`, excluding the `+++` file header.
	pub(crate) Plus:usize,
	/// Lines starting with `-`, excluding the `---` file header.
	pub(crate) Minus:usize,
}
