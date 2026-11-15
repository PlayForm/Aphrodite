//! `GetPreviewFamily`: the registered model family.

use crate::Template::State::Fn::Fn as State;

/// Currently registered `[previews] model_family` selector.
///
/// `None` means "unset", not "no templates": the render path substitutes
/// `"code_first"` for it. Reporting the substituted value here would make a
/// restore impossible to verify, because the previous value would already have
/// been lost.
pub fn Fn() -> Option<String> { State().read().unwrap_or_else(std::sync::PoisonError::into_inner).Family.clone() }
