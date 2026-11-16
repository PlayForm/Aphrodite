//! Why a resolution stopped before producing content.

/// Why the walk has no content to hand back.
///
/// The walk used to answer `None` for both of these, so "the store does not hold
/// this hash" and "a bounded expansion abandoned its output" were one value. A nested
/// reference has to tell them apart: an absent hash leaves its ORIGINAL marker text in
/// place, because a merely evicted or late entry can still heal (F1), while an
/// abandoned expansion must stop the whole expansion - inlining a document that still
/// carries a marker is exactly what the auto-expand budget exists to prevent.
///
/// A `Result<Expansion, Halt>` also keeps `Option` out of the recursion, where `None`
/// had three meanings at once and every caller had to be read to know which one it was
/// handling.
pub(crate) enum Enum {
	/// The store does not hold the hash.
	Absent,
	/// A bounded traversal would have exceeded its budget, so its output was abandoned
	/// mid-construction and no text exists to return. `Budget` names the cause, not the
	/// effect: `Nested::Enum::Abandoned` is what a nested reference does with this answer.
	Budget,
}
