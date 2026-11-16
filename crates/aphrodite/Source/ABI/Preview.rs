//! `AphroditePreview` - build the honest preview string for content.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional, Required::Fn::Fn as Required},
};

/// Generate preview for content - port of _marker/preview.py
///
/// Stateless, and the same builder the compression path uses, which is the
/// point: a caller that shows one preview and stores another has told the model
/// something the store does not contain. The answer is the preview text itself.
/// # Safety
/// `Content` and `CCRType` must each be null or point to a valid, NUL-terminated
/// C string that stays readable and unmoved for the duration of this call. The
/// returned pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditePreview(Content:*const c_char, CCRType:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let CCRType = unsafe { Cstr(CCRType) };

	Run(move || {
		let Content = Required(Content)?;
		let CCRType = Optional(CCRType)?;

		Ok(Answer::Text(crate::Preview::Builder::Entry::Fn(&CCRType, &Content)))
	})
}
