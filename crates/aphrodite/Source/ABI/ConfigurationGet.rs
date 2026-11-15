//! `AphroditeConfigurationGet` - read one configuration value from the session.

use std::{ffi::CString, os::raw::c_char};

use crate::ABI::Guard::Cstr;
use crate::ABI::Guard::Guarded;
use crate::ABI::Guard::ToJSONError;
use crate::ABI::Handle::Lookup::GetHandle;

/// Answer with one configuration value as a bare C string.
///
/// The key set is closed and deliberately small: an unknown key is an error, not
/// an empty answer, because a host reading a typo'd key would otherwise believe
/// the value is unset. Numbers are rendered as decimal strings, so the caller
/// parses rather than the ABI inventing a type tag.
///
/// Like `AphroditeRetrieve`, success is the value itself - there is no JSON
/// envelope to unwrap.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeConfigurationGet(Handle:*const c_char, Key:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let k = unsafe { Cstr(Key) }.unwrap_or_default();
	Guarded(std::panic::AssertUnwindSafe(move || {
		match GetHandle(Hid) {
			Some(Session) => {
				let s = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
				let v = match k.as_ref() {
					"model" => s.Model.clone(),
					"api_url" => s.APIURL.clone(),
					"engine_threshold_pct" => s.EngineThresholdPct.to_string(),
					"tool_threshold" => s.ToolThreshold.to_string(),
					"context_engine_enabled" => s.ContextEngineEnabled.to_string(),
					_ => return ToJSONError(&format!("unknown key: {}", k)),
				};
				CString::new(v.replace('\0', "")).unwrap().into_raw()
			},
			None => ToJSONError(&format!("invalid handle: {}", Hid)),
		}
	}))
}
