//! What kind of tag document a payload is.

/// A tag document's family.
///
/// `Html` and `Xml` are exclusive by construction, so the unreachable fourth state a pair of
/// booleans would admit - "both html and xml" - cannot be represented. It is not a validator:
/// a truncated document that opens and closes still classifies, because the arm only needs to
/// know which preview to render.
pub(crate) enum Enum {
	Html,
	Xml,
	Neither,
}
