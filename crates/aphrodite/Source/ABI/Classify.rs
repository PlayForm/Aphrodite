//! `AphroditeClassify` - stateless content-type detection.

use std::os::raw::c_char;

use headroom_core::transforms;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::{Content::Fn::Fn as CheckContent, Cstr::Fn::Fn as Cstr},
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
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeClassify(Content:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };

	Run(move || {
		let Content = Content.ok_or_else(|| anyhow::anyhow!("null content"))?;

		CheckContent(&Content)?;

		let Ct = transforms::content_detector::detect_content_type(&Content).content_type;

		Ok(Answer::Envelope(
			serde_json::json!({"type":Ct.as_str(),"lines":Content.lines().count(),"bytes":Content.len()}),
		))
	})
}
