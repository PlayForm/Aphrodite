//! `AphroditeFilterLines` - narrow stored content to the lines matching a
//! query.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Run::Fn::Fn as Run},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Filter lines by query - port of _resolve.py _filter_lines
///
/// The answer is the filtered text itself, not a JSON envelope, because it is
/// meant to be spliced straight back into context. Stateless: it filters what it
/// is given and stores nothing, so a caller that wants the narrowed view
/// preserved has to compress it again.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeFilterLines(Content:*const c_char, Query:*const c_char) -> *mut c_char {
	let Content = unsafe { Cstr(Content) };
	let Query = unsafe { Cstr(Query) }.unwrap_or_default();

	Run(move || {
		let Content = Content.ok_or_else(|| anyhow::anyhow!("null content"))?;

		Ok(Answer::Text(crate::Resolve::Filter::Fn(&Content, &Query)))
	})
}
