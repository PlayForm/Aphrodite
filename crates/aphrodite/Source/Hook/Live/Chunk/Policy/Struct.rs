//! How live-container content is cut into chunks.

use std::num::NonZeroUsize;

use super::super::Iter::Struct::Struct as Iter;

/// The chunk policy: a TARGET size, cut at or beyond it.
///
/// The target is not a maximum, and saying which one it is matters: a chunk is
/// closed at the first UTF-8 char boundary whose byte index reaches or exceeds
/// the target, so a chunk may be up to three bytes LARGER than the target when a
/// multi-byte character straddles the boundary, and never smaller (except the
/// final chunk, which is whatever remains). A strict maximum would have to
/// decide what to do with a single scalar larger than the maximum; a target
/// never has to, which is why the existing behaviour is the one named here.
///
/// `Target` cannot be zero. A zero target would close a chunk at every character
/// - the pathological case the old `Vec<&str>` chunker expressed by producing one
///   empty chunk per character - so the type refuses it instead of leaving each
///   caller to remember.
pub(crate) struct Struct {
	Target:NonZeroUsize,
}

impl Struct {
	/// The policy for `Target` bytes, or `None` when `Target` is zero.
	pub(crate) fn new(Target:usize) -> Option<Struct> { NonZeroUsize::new(Target).map(|Target| Struct { Target }) }

	/// The target size in bytes, as the metadata envelope reports it.
	pub(crate) fn Target(&self) -> usize { self.Target.get() }

	/// Cut `Content` under this policy.
	pub(crate) fn Chunks<'a>(&self, Content:&'a str) -> Iter<'a> { Iter::new(Content, self.Target) }
}
