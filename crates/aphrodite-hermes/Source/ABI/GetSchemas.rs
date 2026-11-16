//! `aphrodite_hermes_get_schemas` - every schema as a JSON array.

use std::os::raw::c_char;

use crate::{
	Guard::{Fn::guarded, ToJSON::Fn::Fn as to_json},
	Schema,
};

/// Return all tool schemas as a JSON array.
///
/// One renderer for every JSON payload: a serialisation failure arrives as error
/// JSON rather than as an empty string the caller cannot tell from "no tools".
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_schemas() -> *mut c_char {
	guarded(|| to_json(&serde_json::json!(Schema::All::Fn::Fn())))
}
