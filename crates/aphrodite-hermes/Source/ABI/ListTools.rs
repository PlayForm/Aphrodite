//! `aphrodite_hermes_list_tools` - every registered tool schema.

use std::os::raw::c_char;

use crate::{
	Guard::{Fn::guarded, ToJSON},
	Schema,
};

/// List all registered Hermes tool schemas as JSON array.
///
/// Rendered by the one JSON adapter, so a serialisation failure is error JSON
/// instead of the empty string `unwrap_or_default()` used to hand the caller.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_list_tools() -> *mut c_char {
	guarded(|| ToJSON::Fn::Fn(&serde_json::json!(Schema::All::Fn::Fn())))
}
