//! `aphrodite_hermes_get_schemas` - every schema as a JSON array.

use std::os::raw::c_char;

use crate::{
	Guard::Fn::{guarded, to_c_string},
	Schema,
};

/// Return all tool schemas as a JSON array.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_schemas() -> *mut c_char {
	guarded(|| {
		let schemas = Schema::All::Fn::Fn();
		to_c_string(&serde_json::json!(schemas).to_string())
	})
}
