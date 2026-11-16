//! `AphroditeFilterLines` - narrow stored content to the lines matching a
//! query.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional, Required::Fn::Fn as Required},
};

/// Filter lines by query - port of _resolve.py _filter_lines
///
/// The answer is the filtered text itself, not a JSON envelope, because it is
/// meant to be spliced straight back into context. Stateless: it filters what it
/// is given and stores nothing, so a caller that wants the narrowed view
/// preserved has to compress it again.
/// # Safety
/// `Content` and `Query` must each be null or point to a valid, NUL-terminated C
/// string that stays readable and unmoved for the duration of this call. The
/// returned pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeFilterLines(Content:*const c_char, Query:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let Query = unsafe { Cstr(Query) };

	Run(move || {
		let Content = Required(Content)?;
		let Query = Optional(Query)?;

		Ok(Answer::Text(crate::Resolve::Filter::Fn(&Content, &Query)))
	})
}
