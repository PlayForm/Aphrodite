//! What the walk already knows about a hash, before it resolves anything.

/// The prelude's verdict for one hash.
///
/// The walk asks three questions in a fixed order - is this hash already on the
/// path, is the depth cap reached, was it resolved earlier in this tree - and each
/// one either stops the walk with a different answer or lets it continue. Spelled
/// as three tests at the top of the walk they were three `match <bool>` blocks
/// with a no-op arm apiece, so the order they had to be asked in was implicit in
/// the statement sequence and the reasons to stop had no names.
///
/// The variants ARE those reasons, which is what lets the walk be one structural
/// match: `Revisit` and `Cached` differ in whether the cache is even consulted,
/// `AtCap` answers the entry unresolved, and `Fresh` is the only case that costs a
/// store read.
pub(crate) enum Enum {
	/// Already on the current path: answer from the cache, which may still miss.
	/// This is the cycle guard - without it a self-referential entry recurses
	/// until the stack runs out.
	Revisit,
	/// The configured depth cap is reached: answer the entry as it is stored,
	/// without expanding it further. Falling back to the cache here would answer
	/// `None` for a hash that legitimately exists but has not been visited yet.
	AtCap,
	/// Resolved earlier in this tree: reuse it rather than resolve it twice.
	Cached(String),
	/// Nothing is known: resolve, scan and substitute.
	Fresh,
}
