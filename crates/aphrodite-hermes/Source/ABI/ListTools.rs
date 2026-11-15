//! `aphrodite_hermes_list_tools` - every registered tool schema.

use std::os::raw::c_char;

use crate::{
	Guard::Fn::{guarded, to_c_string},
	Schema,
};

/// List all registered Hermes tool schemas as JSON array.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_list_tools() -> *mut c_char {
	guarded(|| {
		let schemas = Schema::All::Fn::Fn();
		to_c_string(&serde_json::to_string(&schemas).unwrap_or_default())
	})
}
