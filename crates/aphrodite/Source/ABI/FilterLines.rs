//! `AphroditeFilterLines` - narrow stored content to the lines matching a
//! query.

use std::{ffi::CString, os::raw::c_char};

use super::Guard::{Cstr, Guarded, ToJSONError};

/// Filter lines by query - port of _resolve.py _filter_lines
///
/// The answer is the filtered text itself, not a JSON envelope, because it is
/// meant to be spliced straight back into context. Stateless: it filters what it
/// is given and stores nothing, so a caller that wants the narrowed view
/// preserved has to compress it again.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeFilterLines(Content:*const c_char, Query:*const c_char) -> *mut c_char {
	let c = match unsafe { Cstr(Content) } {
		Some(s) => s,
		None => return ToJSONError("null content"),
	};
	let q = unsafe { Cstr(Query) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		let Filtered = crate::Resolve::Filter::Fn(&c, &q);
		CString::new(Filtered.replace('\0', "")).unwrap().into_raw()
	}))
}
