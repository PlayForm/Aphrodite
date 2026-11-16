//! What one marker-shaped region of a document turned out to be.

/// The verdict on one candidate region of a document.
///
/// Every region that begins at an opening delimiter is exactly one of these, and each
/// answer is a decision about text the resolver did not write - which is why they are
/// named here rather than left implicit in the scanner's loop. See
/// [`super::super::Tolerance`] for the policy these implement and for the fourth case
/// (a reference the store does not hold), which is not a grammar question.
pub(crate) enum Enum<'a> {
	/// A complete marker naming a hash: the scanner emits it and the walk resolves it.
	Reference { Marker:&'a str, Hash:&'a str, End:usize },
	/// A complete marker naming no hash (`<<<CCR:>>>`, `<<<CCR:|t|1>>>`): there is
	/// nothing to look up, so it is not an occurrence and the scan continues after it.
	Hashless { End:usize },
	/// An opening delimiter with no closing one: the writer never finished emitting it,
	/// so guessing at its extent would resolve text nobody wrote. The scan steps over
	/// the delimiter and keeps looking.
	Unterminated,
}
