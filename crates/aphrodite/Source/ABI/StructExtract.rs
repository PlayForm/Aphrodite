//! `AphroditeStructExtract` - pull the structural outline out of source code.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional, Required::Fn::Fn as Required},
};

/// Code structure extraction - port of _core/struct.py
///
/// `Language` selects the extractor; an unrecognised language is answered by the
/// extractor itself rather than rejected here, so the caller sees the same
/// fallback shape it would get from a guess. The answer is JSON-wrapped because
/// the result is a structure (names, kinds, line spans), not text to splice.
/// # Safety
/// `Content` and `Language` must each be null or point to a valid,
/// NUL-terminated C string that stays readable and unmoved for the duration of
/// this call. The returned pointer is owned by the caller and must be released
/// with `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeStructExtract(Content:*const c_char, Language:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let Language = unsafe { Cstr(Language) };

	Run(move || {
		let Content = Required(Content)?;
		let Language = Optional(Language)?;

		Ok(Answer::JSON(serde_json::json!(crate::StructExtract::Extract::Fn(
			&Content, &Language,
		))))
	})
}
