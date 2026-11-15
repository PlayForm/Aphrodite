//! `aphrodite_hermes_dispatch_tool` - route a tool call by name.

use std::os::raw::c_char;

use crate::{
	Guard::Fn::{cstr_to_string, guarded, to_c_string, to_json_error},
	Tool,
};

/// Dispatch an aphrodite tool call by name.
/// Returns JSON result string. Caller must free with
/// aphrodite_hermes_free_string.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_dispatch_tool(tool_name:*const c_char, args_json:*const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(tool_name) };
	let args = unsafe { cstr_to_string(args_json) };

	guarded(std::panic::AssertUnwindSafe(move || {
		let result = Tool::Dispatch::Fn(&name, &args);
		match serde_json::to_string(&result) {
			Ok(json) => to_c_string(&json),
			Err(e) => to_json_error(&format!("serialize error: {}", e)),
		}
	}))
}
