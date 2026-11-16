//! `Template::Family::Set::Fn`: register the model family.

use crate::Template::State::Fn::Fn as State;

/// Register the `[previews] model_family` selector; None = default
/// (`"code_first"`). Returns the previous value (restore-friendly).
///
/// Stored as the raw option rather than resolved to `"code_first"` here: the
/// default is applied at RENDER time, so a caller can tell "unset" from
/// "explicitly code_first" and a restore puts back exactly what was there.
pub fn Fn(Family:Option<String>) -> Option<String> {
	let mut St = State().write().unwrap_or_else(std::sync::PoisonError::into_inner);
	std::mem::replace(&mut St.Family, Family)
}
