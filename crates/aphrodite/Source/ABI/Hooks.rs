//! `AphroditeHooks` - discovery of the hook names this engine serves.

use std::os::raw::c_char;

use crate::ABI::Boundary::{Answer::Enum::Enum as Answer, Transfer::Fn::Fn as Transfer};

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
///
/// Takes no pointer, so it stays a safe function. The string still leaves through
/// `Boundary::Transfer` like every other payload this ABI allocates, which is what
/// makes "one allocator, one deallocator" true of the whole surface.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeHooks() -> *mut c_char {
	Transfer(Answer::Text(
		serde_json::json!([
			"on_session_start",
			"pre_tool_call",
			"transform_tool_result",
			"transform_terminal_output",
			"pre_llm_call",
			"post_llm_call"
		])
		.to_string(),
	))
}
