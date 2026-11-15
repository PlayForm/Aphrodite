//! The two shapes a successful export answers with.

/// A successful operation's payload, before it becomes a pointer.
///
/// The ABI has exactly two success shapes and they are not interchangeable:
/// most exports answer with a JSON envelope a host parses, while
/// `AphroditeRetrieve`, `AphroditeConfigurationGet`, `AphroditeFilterLines`,
/// `AphroditePreview`, `AphroditeStage2` and `AphroditeInit` answer with the
/// value itself because the caller's next move is to splice it back into
/// context or to parse a decimal handle. Wrapping those in a `Value::String`
/// would JSON-escape the payload and change every byte the caller receives, so
/// the difference is carried here rather than erased.
pub(crate) enum Enum {
	/// A JSON envelope, rendered as `Value::to_string()`.
	JSON(serde_json::Value),
	/// Raw text, rendered as-is (an interior NUL is stripped by `Transfer`).
	Text(String),
}
