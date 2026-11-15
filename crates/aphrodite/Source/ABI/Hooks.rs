//! `AphroditeHooks` - discovery of the hook names this engine serves.

use std::{ffi::CString, os::raw::c_char};

/// Report the hook names the engine implements, as a JSON array of strings.
///
/// `OnSessionStart`, not `session_start`: Hermes's VALID_HOOKS table
/// requires the `on_` prefix - registering the bare name silently no-ops
/// in Hermes (the bridge crate already returns the correct name; this
/// discovery fn used to hand out the broken one). `AphroditeDispatch`/
/// `AphroditeCallHook` accept both names for compatibility.
///
/// The list is what a host registers against, so a name that is not in this
/// array is a hook that will never fire.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeHooks() -> *mut c_char {
	CString::new(
		serde_json::json!([
			"on_session_start",
			"pre_tool_call",
			"transform_tool_result",
			"transform_terminal_output",
			"pre_llm_call",
			"post_llm_call"
		])
		.to_string(),
	)
	.unwrap()
	.into_raw()
}
