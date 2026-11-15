//! One markdown heading.

/// A heading line reduced to its level and its text.
///
/// `Text` is the trimmed heading line INCLUDING its leading hashes, which is what the
/// preview has always shown - the model reads the level twice, and that redundancy is
/// cheaper than a second field nobody asked for. It is not a section: the body under the
/// heading is not attributed to it.
pub(crate) struct Struct {
	pub Level:usize,
	pub Text:String,
}
