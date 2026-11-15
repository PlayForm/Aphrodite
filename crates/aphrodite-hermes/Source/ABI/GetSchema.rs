//! `aphrodite_hermes_get_schema` - one tool schema by name.

use std::os::raw::c_char;

use crate::{
	Guard::Fn::{cstr_to_string, guarded, to_c_string, to_json_error},
	Schema,
};

/// Get a single tool schema by name.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_schema(tool_name:*const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(tool_name) };
	guarded(std::panic::AssertUnwindSafe(move || {
		match Schema::Get::Fn::Fn(&name) {
			Some(s) => to_c_string(&serde_json::to_string(&s).unwrap_or_default()),
			None => to_json_error(&format!("unknown tool: {}", name)),
		}
	}))
}
