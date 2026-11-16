//! `aphrodite_hermes_get_schema` - one tool schema by name.

use std::os::raw::c_char;

use crate::{
	Guard::{
		Fn::{cstr_to_string, guarded, to_json_error},
		ToJSON::Fn::Fn as to_json,
	},
	Schema,
};

/// Get a single tool schema by name.
///
/// A known name renders through the one JSON adapter; an unknown name is error
/// JSON. A serialisation failure is error JSON too - never an empty string,
/// which is a valid C string that reads as "the tool has no schema".
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_schema(tool_name:*const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(tool_name) };
	guarded(std::panic::AssertUnwindSafe(move || {
		match Schema::Get::Fn::Fn(&name) {
			Some(Found) => to_json(&Found),
			None => to_json_error(&format!("unknown tool: {}", name)),
		}
	}))
}
