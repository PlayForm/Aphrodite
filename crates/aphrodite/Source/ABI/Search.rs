//! `AphroditeSearch` - find already-stored entries by keyword.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Answer with up to twenty stored entries whose preview or type contains
/// `Query`, matched case-insensitively.
///
/// A search hit is a hash the caller still has to retrieve; the result set is
/// capped at twenty because the answer is meant to be read, not paged. Matching
/// against the preview is what makes the result usable - the stored bytes are
/// not searched, so a keyword that only appears deep inside a payload will not
/// match.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeSearch(Handle:*const c_char, Query:*const c_char) -> *mut c_char {
	let Query = unsafe { Cstr(Query) }.unwrap_or_default().to_lowercase();

	Bare(Handle, move |s| {
		let Results:Vec<serde_json::Value> = s
			.RecentMarkers
			.iter()
			.filter(|Marker| {
				Marker.Preview.to_lowercase().contains(&Query) || Marker.CCRType.to_lowercase().contains(&Query)
			})
			.take(20)
			.map(|Marker| {
				serde_json::json!({
					"hash":&Marker.hash,
					"type":Marker.CCRType,
					"size":Marker.Size,
					"preview":Marker.Preview,
				})
			})
			.collect();

		Ok(Answer::JSON(serde_json::json!({"total":Results.len(),"results":Results})))
	})
}
