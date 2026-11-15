//! `AphroditeStats` - the session's own counters, plus its configuration
//! self-diagnosis.

use std::os::raw::c_char;

use crate::ABI::Guard::Cstr;
use crate::ABI::Guard::Guarded;
use crate::ABI::Guard::ToJSONError;
use crate::ABI::Guard::ToJSONOk;
use crate::ABI::Handle::Lookup::GetHandle;

/// Answer with the session's counters and the configuration actually in
/// effect.
///
/// `ConfigurationError` is the field that matters when the numbers look wrong: it is
/// present exactly when an `aphrodite.toml` was found but failed to parse, so
/// the caller is reading defaults rather than what it configured. A host that
/// only logs the counters loses the one signal that explains them.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeStats(Handle:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	Guarded(std::panic::AssertUnwindSafe(move || {
		match GetHandle(Hid) {
			Some(Session) => {
				let s = Session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
				ToJSONOk(&serde_json::json!({
					"version":env!("CARGO_PKG_VERSION"),"inline_entries":s.InlineStore.len(),
					"markers":s.RecentMarkers.len(),"turn":s.TurnCounter,
					"engine_enabled":s.ContextEngineEnabled,"threshold_pct":s.EngineThresholdPct,
					"tool_threshold":s.ToolThreshold,"terminal_threshold":s.TerminalThreshold,
					"auto_reload":s.AutoReload,
					// Config self-diagnosis: present when aphrodite.toml was
					// found but failed to parse - defaults are in effect.
					"config_error":s.ConfigurationError,
				}))
			},
			None => ToJSONError(&format!("invalid handle: {}", Hid)),
		}
	}))
}
