//! `AphroditeStructExtract` - pull the structural outline out of source code.

use std::os::raw::c_char;

use super::Guard::{Cstr, Guarded, ToJSONError, ToJSONOk};

/// Code structure extraction - port of _core/struct.py
///
/// `Language` selects the extractor; an unrecognised language is answered by the
/// extractor itself rather than rejected here, so the caller sees the same
/// fallback shape it would get from a guess. The answer is JSON-wrapped because
/// the result is a structure (names, kinds, line spans), not text to splice.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeStructExtract(Content:*const c_char, Language:*const c_char) -> *mut c_char {
	let c = match unsafe { Cstr(Content) } {
		Some(s) => s,
		None => return ToJSONError("null content"),
	};
	let LanguageTag = unsafe { Cstr(Language) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		let Result = crate::StructExtract::Extract::Fn(&c, &LanguageTag);
		ToJSONOk(&serde_json::json!(Result))
	}))
}
