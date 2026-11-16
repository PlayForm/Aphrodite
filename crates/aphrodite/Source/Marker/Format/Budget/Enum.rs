//! How much of a preview the headroom budget keeps.

/// The truncation a headroom budget asks for.
///
/// The budget is a percentage the caller supplies, and only three of its ranges
/// mean anything: the rest mean "do not truncate". Naming that third outcome
/// `Unlimited` is what removes the `Some(75..)` arm that used to read as a
/// no-op inside a `match` on the number.
pub(crate) enum Enum {
	/// Keep the preview as it is.
	Unlimited,
	/// Keep at most this many characters.
	At(usize),
}
