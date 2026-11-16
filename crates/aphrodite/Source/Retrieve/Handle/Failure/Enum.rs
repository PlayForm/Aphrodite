//! Why a retrieve could not serve content.

/// Every way `/retrieve` can fail, as one value.
///
/// The handler used to build four different failure bodies at four different
/// `return` sites, each with its own copy of the six response fields, so a change
/// to the envelope had four places to land in and the status code was decided
/// away from the body it belonged to. The variants are the four decisions, and
/// [`super::Reply::Fn::Fn`] is the one place that renders them.
pub(crate) enum Enum {
	/// The request carried no usable hash (`hash` absent, or empty after
	/// normalisation). It is not a lookup failure: nothing was asked for.
	Missing,
	/// The inline store's mutex is poisoned. That is a broken process, not a
	/// cache miss - serving it as a 404 sent callers looking for a missing entry
	/// that was never the problem.
	Poisoned,
	/// Neither the inline store nor the CCR backend holds the hash. It is not
	/// an empty entry: an entry that exists with empty content is found.
	Absent(String),
	/// The line window was refused (an offset at or past the end of the
	/// document). The refusal's own message is the response body, because it
	/// names the offset and the document's line count - the two numbers the
	/// caller needs.
	Refused(anyhow::Error),
}
