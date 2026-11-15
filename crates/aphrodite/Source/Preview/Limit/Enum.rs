//! How long a rendered preview may be.

/// The preview length budget.
///
/// `Unlimited` is an absent or zero cap, and `Characters` can never be zero - so
/// "no cap" and "cap of nothing" are two values instead of one ambiguous integer.
/// It is not a per-arm promise: the cap is applied once, by `Render::Cap`, to every
/// path through the entry point.
pub(crate) enum Enum {
	/// No cap at all - the configured value was absent or zero.
	Unlimited,
	/// At most this many characters.
	Characters(std::num::NonZeroUsize),
}
