//! `AphroditeStage2` - the semantic reduction pass over a payload.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Stage 2 semantic reduction - port of _stage2.py
///
/// Returns the reduced text, or `{"error":"no reduction possible"}` when the
/// content has no shape this reducer understands. "No reduction possible" is a
/// normal answer, not a failure of the caller: stage 2 is opportunistic, and a
/// caller that treats it as an error will retry content that will never reduce.
///
/// That is why the refusal is an `anyhow::Error` here and not an `Answer`: the
/// boundary renders it as the same `{"error": ...}` envelope either way, and
/// routing it through `?` keeps the two success shapes on the arms that actually
/// succeed.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeStage2(Content:*const c_char, CCRType:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let CCRType = unsafe { Cstr(CCRType) }.unwrap_or_default();

	Run(move || {
		let Content = Content.ok_or_else(|| anyhow::anyhow!("null content"))?;

		crate::Stage2::Compress::Fn::CompressStage2(&Content, &CCRType)
			.map(Answer::Text)
			.ok_or_else(|| anyhow::anyhow!("no reduction possible"))
	})
}
