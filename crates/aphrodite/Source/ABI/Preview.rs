//! `AphroditePreview` - build the honest preview string for content.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Generate preview for content - port of _marker/preview.py
///
/// Stateless, and the same builder the compression path uses, which is the
/// point: a caller that shows one preview and stores another has told the model
/// something the store does not contain. The answer is the preview text itself.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditePreview(Content:*const c_char, CCRType:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let CCRType = unsafe { Cstr(CCRType) }.unwrap_or_default();

	Run(move || {
		let Content = Content.ok_or_else(|| anyhow::anyhow!("null content"))?;

		Ok(Answer::Text(crate::Preview::Builder::Entry::Fn(&CCRType, &Content)))
	})
}
