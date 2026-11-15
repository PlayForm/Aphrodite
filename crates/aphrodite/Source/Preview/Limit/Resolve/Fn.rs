//! Translate the configured cap into the budget value.

use crate::Preview::Limit::Enum::Enum;

/// Resolve the configured character cap, where `0` means unlimited.
///
/// This is the ONLY place the `0` sentinel is read, so every consumer downstream
/// sees either `Unlimited` or a non-zero count. It is not a validation point: an
/// absurdly small cap is accepted and simply truncates every preview, because
/// refusing the value would leave the previous cap in force with no signal that
/// configuration was ignored.
pub(crate) fn Fn(Maximum:usize) -> Enum {
	std::num::NonZeroUsize::new(Maximum).map_or(Enum::Unlimited, Enum::Characters)
}
