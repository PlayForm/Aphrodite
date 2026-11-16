//! The ` +N more` suffix a bounded list appends.

use std::num::NonZeroUsize;

/// The hidden-item suffix for a list that showed `Shown` of `Total`.
///
/// `saturating_sub` plus `NonZeroUsize` makes "nothing is hidden" the absence of a value rather
/// than a zero every caller has to compare - the same shape `Render::Cap` uses for "it fits", and
/// the reason a list that shows everything cannot render ` +0 more`. It is not a count of
/// anything: the caller has already bounded the list, and this only reports how much of the total
/// that bound hid.
pub(crate) fn Fn(Total:usize, Shown:usize) -> String {
	NonZeroUsize::new(Total.saturating_sub(Shown))
		.map(|More| format!(" +{More} more"))
		.unwrap_or_default()
}
