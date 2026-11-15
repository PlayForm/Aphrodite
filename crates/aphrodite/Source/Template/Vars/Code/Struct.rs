//! The structure map of a code payload, as counts and signatures.

/// What the structure extractor found in a code payload.
///
/// `Fns` is an `Option` because the pre-Rust namespace is asymmetric about absence:
/// a code type whose extractor found no functions renders `"?"` for `{fns}` while the
/// other three render `"0"`. Keeping that asymmetry as a value is what lets the
/// renderer apply it once, instead of an extractor inventing a `"?"` and the renderer
/// having to guess whether it meant it.
///
/// `Default` is the empty record, which is the answer for a NON-code type: there is
/// no structure map to read, so the gate is this value rather than a branch at the
/// call site.
#[derive(Default)]
pub(crate) struct Struct {
	/// How many functions the extractor found, if it found any.
	pub(crate) Fns:Option<usize>,
	/// Struct declarations.
	pub(crate) Structs:usize,
	/// Impl blocks.
	pub(crate) Impls:usize,
	/// Class declarations.
	pub(crate) Classes:usize,
	/// The first two function signatures, joined by `; ` - `{sigs}` renders this and
	/// `{sig1}` renders its first entry.
	pub(crate) Sigs:String,
}
