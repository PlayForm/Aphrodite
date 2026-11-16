//! `Transfer`: an answer as an owned C string.

use std::{ffi::CString, os::raw::c_char};

use super::super::Answer::Enum::Enum as Answer;

/// Allocate the C string the caller owns and will release with
/// `AphroditeFreeString`.
///
/// Every pointer-returning export hands its payload over here, so "who allocated
/// this block" has one answer for the whole ABI.
///
/// A null return is the only failure shape and it means `CString::new` refused the
/// bytes - an interior NUL in a JSON envelope or an error message. That is why
/// `Text` is sanitised here: a stored payload can legitimately carry a NUL (it
/// arrives through `AphroditeDispatch`'s JSON args, which can encode one), and the
/// ABI has always answered with the NUL stripped rather than with a truncated
/// string or a null. The JSON arm is not sanitised because `Value::to_string()`
/// escapes a NUL as `\u0000` and never emits a raw one.
///
/// The `Text` arm only builds a new string when there is a NUL to strip: without
/// one, the payload's own buffer is moved into the `CString`, so the common path
/// copies nothing.
pub(crate) fn Fn(Answer:Answer) -> *mut c_char {
	match Answer {
		Answer::JSON(Value) => CString::new(Value.to_string()).map_or(std::ptr::null_mut(), CString::into_raw),
		Answer::Text(Text) => {
			let Text = match Text.contains('\0') {
				true => Text.replace('\0', ""),
				false => Text,
			};

			CString::new(Text).map_or(std::ptr::null_mut(), CString::into_raw)
		},
	}
}
