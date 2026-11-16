//! `AphroditeClassify` - stateless content-type detection.

use std::os::raw::c_char;

use headroom_core::transforms;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::{
		Content::Fn::Fn as Check,
		Cstr::{Fn::Fn as Cstr, Required::Fn::Fn as Required},
	},
};

/// Classify `Content` and answer with its CCR type, line count and byte size.
///
/// Stateless by design: no handle, no session, nothing stored. It exists for a
/// host that wants the classification before deciding whether to compress at
/// all, and for callers with no session to speak of.
///
/// Null content is a distinct error from empty content - the first means the
/// caller passed no pointer, the second means it passed a zero-length string,
/// and a host debugging its own marshalling needs to tell them apart.
/// # Safety
/// `Content` must be null or point to a valid, NUL-terminated C string that stays
/// readable and unmoved for the duration of this call. The returned pointer is
/// owned by the caller and must be released with `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeClassify(Content:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };

	Run(move || {
		let Content = Required(Content)?;

		Check(&Content)?;

		let Type = transforms::content_detector::detect_content_type(&Content).content_type;

		Ok(Answer::JSON(
			serde_json::json!({"type":Type.as_str(),"lines":Content.lines().count(),"bytes":Content.len()}),
		))
	})
}
