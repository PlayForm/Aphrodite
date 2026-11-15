//! `AphroditeStructExtract` - pull the structural outline out of source code.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Code structure extraction - port of _core/struct.py
///
/// `Language` selects the extractor; an unrecognised language is answered by the
/// extractor itself rather than rejected here, so the caller sees the same
/// fallback shape it would get from a guess. The answer is JSON-wrapped because
/// the result is a structure (names, kinds, line spans), not text to splice.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeStructExtract(Content:*const c_char, Language:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let Language = unsafe { Cstr(Language) }.unwrap_or_default();

	Run(move || {
		let Content = Content.ok_or_else(|| anyhow::anyhow!("null content"))?;

		Ok(Answer::JSON(serde_json::json!(crate::StructExtract::Extract::Fn(
			&Content, &Language,
		))))
	})
}
