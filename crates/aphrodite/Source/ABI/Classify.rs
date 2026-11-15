//! `AphroditeClassify` - stateless content-type detection.

use std::os::raw::c_char;

use headroom_core::transforms;

use super::Guard::{CheckContent, Cstr, Guarded, ToJSONError, ToJSONOk};

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
	let c = match unsafe { Cstr(Content) } {
		Some(s) => s,
		None => return ToJSONError("null content"),
	};
	Guarded(std::panic::AssertUnwindSafe(move || {
		if let Err(e) = CheckContent(&c) {
			return ToJSONError(&format!("{e:#}"));
		}
		let Ct = transforms::content_detector::detect_content_type(&c).content_type;
		ToJSONOk(&serde_json::json!({"type":Ct.as_str(),"lines":c.lines().count(),"bytes":c.len()}))
	}))
}
