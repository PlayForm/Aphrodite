//! Whether a dylib is needed for the install to be usable.

/// How much a missing dylib matters.
///
/// A `bool` carried this and read as `(name, true)` at every use site, which is
/// the kind of unnamed decision a reader has to decode from the call. The two
/// variants are the whole policy: `Optional` warns and continues, `Required`
/// aborts the install with a message naming what to do instead.
#[derive(Clone, Copy)]
pub(crate) enum Enum {
	/// The Hermes plugin loads this one; setup cannot complete without it.
	Required,
	/// Exists for external embedders and is not published as a release asset;
	/// its absence is a warning.
	Optional,
}
