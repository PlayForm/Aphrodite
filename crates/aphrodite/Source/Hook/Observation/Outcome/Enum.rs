//! Whether a call reported success.

/// The outcome a call reported, as a value rather than a boolean.
///
/// Two variants and no third: a call either reported a failure or it did not,
/// and an absent status is success (fail-open). It is an enum because the
/// failure side is what carries the error signature, so the signature and the
/// outcome cannot disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// The call did not report a failure.
	Okay,
	/// The call reported a failure.
	Failed,
}
