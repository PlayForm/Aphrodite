//! `AphroditeSearch` - find already-stored entries by keyword.

use std::os::raw::c_char;

use crate::ABI::Guard::Cstr;
use crate::ABI::Guard::Guarded;
use crate::ABI::Guard::ToJSONError;
use crate::ABI::Guard::ToJSONOk;
use crate::ABI::Handle::Lookup::GetHandle;

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
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let q = unsafe { Cstr(Query) }.unwrap_or_default().to_lowercase();
	Guarded(std::panic::AssertUnwindSafe(move || {
		match GetHandle(Hid) {
			Some(Session) => {
				let s = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
				let Results:Vec<serde_json::Value> = s
					.RecentMarkers
					.iter()
					.filter(|m| m.Preview.to_lowercase().contains(&q) || m.CCRType.to_lowercase().contains(&q))
					.take(20)
					.map(|m| serde_json::json!({"hash":&m.hash,"type":m.CCRType,"size":m.Size,"preview":m.Preview}))
					.collect();
				ToJSONOk(&serde_json::json!({"total":Results.len(),"results":Results}))
			},
			None => ToJSONError(&format!("invalid handle: {}", Hid)),
		}
	}))
}
