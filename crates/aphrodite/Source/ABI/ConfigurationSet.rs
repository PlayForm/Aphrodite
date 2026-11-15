//! `AphroditeConfigurationSet` - write one configuration value on the session.

use std::os::raw::c_char;

use crate::ABI::{
	Guard::{Cstr, ToJSONError, ToJSONOk},
	Handle::Lookup::WithState,
};

/// Set one configuration value and answer with `{"status":"ok"}`.
///
/// Silently ignores an unknown key and an unparseable number: this is a runtime
/// knob, not a validated form, and a host that mistypes a key must not have its
/// session fail. The cost of that leniency is that a caller cannot tell a
/// rejected write from an applied one by the response alone - it has to read the
/// value back with `AphroditeConfigurationGet`.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeConfigurationSet(
	Handle:*const c_char,
	Key:*const c_char,
	Value:*const c_char,
) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let k = unsafe { Cstr(Key) }.unwrap_or_default();
	let v = unsafe { Cstr(Value) }.unwrap_or_default();
	match WithState(Hid, |s| {
		match k.as_ref() {
			"model" => s.Model = v.to_string(),
			"engine_threshold_pct" => {
				match v.parse() {
					Ok(n) => s.EngineThresholdPct = n,
					Err(_) => (),
				}
			},
			"tool_threshold" => {
				match v.parse() {
					Ok(n) => s.ToolThreshold = n,
					Err(_) => (),
				}
			},
			"context_engine_enabled" => s.ContextEngineEnabled = v == "true" || v == "1",
			_ => {},
		}
		serde_json::json!({"status":"ok"})
	}) {
		Ok(v) => ToJSONOk(&v),
		Err(e) => ToJSONError(&format!("{e:#}")),
	}
}
