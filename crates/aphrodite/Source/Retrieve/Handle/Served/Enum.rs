//! Which store answered a retrieve.

/// The store that served a `/retrieve`.
///
/// The handler reported `"source": "ccr"` on every success, so the field the response
/// documents as "which backend served it" could not tell a session's own inline entry from a
/// backend round trip - the two stores the lookup deliberately checks in order, and the whole
/// reason the hit counters exist. The answer is now the lookup's own verdict, so the response
/// cannot disagree with the store that produced it.
///
/// It is not a cache-miss report: a miss has no store to name and is a
/// [`crate::Retrieve::Handle::Failure::Enum::Enum`] instead, which is why this type has no
/// "none" variant.
pub(crate) enum Enum {
	/// The inline store held the hash: no backend round trip was made.
	Inline,
	/// The CCR backend answered.
	Backend,
}

impl Enum {
	/// The wire spelling of this store, which the response's `source` field carries.
	pub(crate) fn Wire(&self) -> &'static str {
		match self {
			Enum::Inline => "inline_ccr",
			Enum::Backend => "ccr",
		}
	}
}
