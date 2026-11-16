//! What one expansion produced.

/// The two states a hash's content can be in when the walk hands it back.
///
/// The walk used to answer `Option<String>`, so "this is the entry as the store
/// holds it" and "this is the entry with every marker substituted" were the same
/// value. They are not the same answer. `Loaded` may still contain markers - it is
/// what the depth cap produces, and what a reference to a hash that is already being
/// expanded further up the same path produces - so nothing may treat it as a
/// finished expansion. `Complete` is the content a caller asked for.
///
/// The distinction is also what lets the cache be honest: it holds `Loaded` for a
/// hash that is still in progress and `Complete` for one that finished, so a cycle
/// and a diamond are answered from the same map without either being mistaken for
/// the other.
///
/// `Clone` because the cache hands out a copy of what it already knows: a cached
/// entry is reused by value at the site that needs it, so the walk never has to hold
/// a borrow of the cache across a recursive call.
#[derive(Clone)]
pub(crate) enum Enum {
	/// The entry as the store holds it, with no marker substituted. Produced by the
	/// depth cap and by a reference to a hash already on the current path.
	Loaded(String),
	/// The entry with every marker that resolved substituted into it. A marker that
	/// did not resolve is still in place: leaving it is what lets it heal.
	Complete(String),
}

impl Enum {
	/// The text either state carries.
	pub(crate) fn Text(&self) -> &str {
		match self {
			Enum::Loaded(Text) | Enum::Complete(Text) => Text,
		}
	}

	/// Consume the state and take its text. `Owned` is the counterpart of `Text`:
	/// the same content, moved out instead of borrowed.
	pub(crate) fn Owned(self) -> String {
		match self {
			Enum::Loaded(Text) | Enum::Complete(Text) => Text,
		}
	}
}
